# RapidR IDE: the master plan (Phase 3)

A professional IDE in the line of RapidQ, VB6, Delphi / Lazarus, Xojo and Xcode: an MDI workspace with docking panels, a WYSIWYG form designer with smart guides, a fast code editor with real IntelliSense, a debugger, live editing, Delphi-style linked data components (with the data-science stack first-class), and AI through MCP. One codebase on the desktop and in the browser, built on RapidR's own UI kernel, and assembled from public components that users can put in their own programs.

This document is the plan only: no code was changed to write it. It is self-contained (it doesn't rely on anyone's memory of earlier conversations); the rules it follows are in §1. Companion documents:

- [docs/ide-components.md](ide-components.md): the public components' APIs (properties, methods, events), language definitions, the extension model, packaging and sandboxing.
- [docs/ide-ai.md](ide-ai.md): the provider layer, keys, the IDE's MCP server and its tools, permissions, data privacy, the threat model, RAI.
- [docs/q-and-r-components.md](q-and-r-components.md): how RapidQ's Q components and RapidR's R components coexist, and the "RapidQ-compatible project" setting.

Written 2026-10-05 from `development` @ `8dc254a` (v2.116.0). Stage results get appended at the end, as in [docs/desktop-host-plan.md](desktop-host-plan.md) and [docs/web-host-plan.md](web-host-plan.md).

---

## 0. Summary

| Stage | What | Size | Sessions | Lanes | In the first public release? |
|---|---|---|---|---|---|
| **I0** Foundations | The language registry (one source of truth), the project format, the program session protocol, a parser fit for tools | L | 12–16 | 4 | Yes |
| **I1** Shell | MDI workspace + docking, project tree, toolbox, inspector, themes, commands, run / stop; one codebase on desktop and web; the HTML IDE deleted | L | 16–22 | 4 | Yes |
| **I2** Code editor | Our own on the kernel: rope buffer, multi-cursor, folding, find / replace, large files, IME, accessibility, declarative languages | L | 14–20 | 3 | Yes |
| **I3** Language service | Completion of locals / globals / functions / components / members, hover, signature help, definition, references, rename, live diagnostics, outline; LSP | L | 14–20 | 3 | Yes |
| **I4** Visual designer | WYSIWYG drag and drop, grid snapping, smart guides (centring, alignment, equal spacing), multi-select, align / distribute, anchors, tab order, menu editor, undo, two-way CREATE sync | L | 16–22 | 4 | Yes |
| **I5** Live | Live property preview into the running program, hot reload, edit-and-continue | M | 8–12 | 2 | No (1.1) |
| **I6** Debugger | Breakpoints (conditional, hit counts, logpoints), watches and data tips evaluated by the VM, call stack, stepping, immediate window, DAP | M | 8–12 | 2–3 | Basic part yes |
| **I7** Linked data & data science | Non-visual component tray; connection / file → dataset / data frame → transforms → data source → data-aware controls and RPlot, live at design time; a data preview panel | L | 18–26 | 4 | Core yes (read-only, live) |
| **I8** Smart | The IDE's MCP server, a floating assistant, diffs before apply, permission levels, multi-provider over plain HTTPS, keys in the OS keychain, data privacy levels, RAI | L | 16–24 | 4 | No (1.1; MCP read-only server as a stretch) |
| **I9** Extensions | Languages, providers and commands written in RapidR, packaged, permissioned and sandboxed | M | 8–12 | 2 | No |

A session is one focused agent session ending in a green commit (the unit the host plans use). Sizes: **S** ≤ 4 sessions, **M** 5–12, **L** more. Total about 130–185 sessions; the first public release (§8) needs about 95–125 of them, which at 4–6 parallel lanes is the "Q1 2027" line of ROADMAP's timeline if work starts as RapidQ compatibility closes.

The architecture in one paragraph: every IDE building block is a **public RapidR component** (a shared model in `rapidr-value`, drawn by `rapidr-ui-kernel`, described for accessibility and AI), so the desktop (winit host) and the browser (canvas host) run the same IDE. The IDE's **shell is a RapidR program** assembled from those components (as `examples/ide.rr` already is, but on the new components), so every feature the IDE needs is a feature users get. The **program under development runs in its own process** (desktop) or **sandboxed frame** (web), driven through one **program session protocol**; its forms appear as windows in the workspace. Services — the language registry and service, the AI provider layer, the MCP server, keys — are Rust crates exposed as non-visual components.

---

## 1. Rules this plan follows

These are the project's standing rules, restated so this plan stands alone.

1. **RapidQ-exact.** Anything RapidQ has behaves exactly as RapidQ; RapidR's extensions are additive and never change old behaviour. **IDE features never change program semantics**: the designer writes source a compiler reads like any other, the formatter never changes meaning, hot reload only applies what a restart would.
2. **One implementation, no fallbacks.** When a replacement is solid, the old path is deleted completely in the same step. The HTML / Monaco web IDE (`web-ide/`) is deleted when I1 reaches parity (§3, I1 acceptance); `examples/ide.rr` is replaced by the new shell. No release ships an "old IDE" switch.
3. **One source of truth** (ROADMAP principle 3). A single language registry generates IDE completion data, the VS Code data, the manual's reference sections and the AI system prompt (I0).
4. **One UI kernel, desktop and web alike** (principles 7 and the web host plan). The IDE is drawn by the kernel on both hosts; the web is the canvas host (the only web host). Web parity is a requirement, not a follow-up: every stage's acceptance runs on both.
5. **Licences.** Anything shipped (in RapidR, the IDE, or a user's program) is under MIT, Apache-2.0, BSD, ISC, Zlib, Unicode-3.0 or CC0, fonts under the SIL OFL. `deny.toml` also admits a few others for existing dependencies (BSL-1.0, 0BSD, Unlicense, CDLA-Permissive-2.0 for webpki-roots' data, MPL-2.0 file-level for three crates); **new IDE dependencies stay inside the stricter list above**. `THIRD_PARTY_NOTICES.md` is regenerated (`tools/third_party_notices.py`) and `tools/regress.sh legal` passes with every new crate. No vendor SDKs for AI.
6. **Accessible by default** (principle 8): the IDE is fully usable with a keyboard and a screen reader.
7. **Secure by default** (principle 4): untrusted code, imported projects and AI output never get more privilege than they asked for.
8. **Local verification.** No remote CI (it costs money); `tools/regress.sh` is the gate. Each stage adds its suites there. Real printers are never used by tests (`RAPIDR_PRINT_TO`).
9. **Close gaps before new work.** A stage isn't done while one runtime (native, interpreter, web) lags.

---

## 2. What exists (findings from the code, 2026-10-05)

| Piece | Where | State | Fate |
|---|---|---|---|
| Web IDE | `web-ide/` (host.js 4,999, ide.css 1,673, lang-data.js 874, monaco-host.js 594, model.js 393, toolbox.js 226, preview.html 181, zip.js 271, …; ~10,000 lines) + vendored Monaco 0.52.2 | Monaco editor, regex IntelliSense, designer (8-px snap, rubber band, 8 handles, align / size / centre / z-order, no guides, no menu editor), property grid (typed editors from hand tables), events via Object / Event drop-downs, assets manager, snapshot undo (100 steps), debugger panels over a `MessageChannel` to the preview iframe, `.rrproj` JSON v1 | **Replaced by I1–I4 and deleted** at parity. Kept until then as the harness; its suites (`tests/web_ide_*.mjs`, 27) become the parity checklist |
| Desktop IDE in RapidR | `examples/ide.rr` (2,778 lines), launched by `rapidr ide` (`crates/rapidr-cli/src/launch.rs`), shipped as `ide/rapidr-ide.rrbc` | One fixed 1200 × 820 form: toolbar of buttons, RTREEVIEW project tree, RDESIGNSURFACE / RCODEEDITOR toggled, two RSTRINGGRID property / event grids, error list, global arrays limited to 100 components; generates `CREATE Form1 AS RFORM`; builds by shelling out to `rapidr` | **Replaced by the I1 shell** (same launch path, `rapidr ide`) |
| RCODEEDITOR | model `rapidr-value/src/objects/{code,textedit}.rs`; kernel `components/{codeedit,memo}.rs` + `text/editor.rs` (1,097) | A memo with a gutter: `Vec<char>` buffer, one selection, one-step undo, no redo / find / folding / multi-cursor / brackets / completion; BASIC colours hard-coded (unreadable in the dark theme); one parley layout per paragraph (only edited paragraphs re-laid, only visible drawn); IME; accessible as one multiline text input; API GetSubList / GotoSub / GotoLine | **Grows into I2's editor** (same name, the old API kept) |
| RDESIGNSURFACE | `objects/design.rs` (749), kernel `components/design.rs` (173) | Placeholders (not real components), single selection, 8-px grid, 3 resize grips, events Select / DblClick / BgClick / Move, accessible as a list box | **Kept as a name** over I4's designer model (its API answered by a thin layer); placeholders replaced by real drawing |
| MDI | `rapidr-value/src/mdi.rs` (737), kernel `components/mdi.rs` (212) | QFORMMDI: floating / minimized / maximized children, cascade, tile, activation; one component per child; no docking | **Reused** as the document area of I1's dock manager |
| QDOCKFORM | `rapidr-preprocessor/src/libraries/QDockForm.inc` (579, BASIC) | One panel docked to an edge or torn off | Unchanged (RapidQ library); not the IDE's docking |
| Themes | `rapidr-value/src/theme.rs` (846) | ~60 tokens, Classic / Fluent looks; CLASSIC, MODERN, DARK, HIGH_CONTRAST; `$THEME`, `Application.Theme` | **Reused**; I1 adds editor token colours and IDE chrome tokens |
| Kernel components | `rapidr-ui-kernel/src/components/` (34 kinds) | Tree, list view, grid, list, combo, tab control, menus, splitter, status bar, scroll box, panel, edits, canvas, … No toolbar kind (RTOOLBAR is a placeholder container) | **Reused** by every panel; I1 adds RTOOLBAR proper |
| Language data | `web-ide/lang-data.js` (68 components, 106 builtins), `utilities/vscodeext/.../languageData.js` (62, 90): hand-written, drifted; Rust lists `rapidr-ast::COMPONENT_TYPES` (88), `rapidr-bytecode::builtins::BUILTINS` (184, test-checked against both hosts), `rapidr_value::component_defaults` | **Replaced by I0's registry**; the two JS files become generated, then the web one is deleted with `web-ide/` |
| Q → R names | `rapidr-ast::canonical_type_name` | `Q` + name → `R` + name when that R component exists (any R component, so `QPLOT` is accepted as RPLOT), QGAUGE → RPROGRESSBAR, QOUTLINE → RTREEVIEW, COMPORT, include-library components decided by the parser | Kept; the registry records which components RapidQ really has (§4, I0) |
| Parser | `rapidr-lexer` (1,387), `rapidr-parser` (3,762), `rapidr-ast` (~6,400), `rapidr-preprocessor` (1,272) | Byte spans on every node, but into the **preprocessed** text (line map only); comments dropped by the lexer (not lossless); whole-file lexing, first lex error aborts; the parser recovers line by line (`parse_tokens_recovering`); symbol tables private to bcgen | **Extended in I0** (comments and directives kept as trivia, byte-level origin map, lexer recovery); a semantic model for I3 |
| VM debugging | `interpreter/rapidr-vm` (`exec_loop`, `debug_mode`, `breakpoints: HashSet<u32>`, `StepMode`), web `DebugSession` (wasm) | Line breakpoints, step in / over / out, stack, locals / globals; no pause-on-demand, conditions, hit counts, per-file breakpoints, writing variables, expression evaluation, hot swap; the web compile path fills no source map (`$INCLUDE` lines off); watches evaluated in JS by name lookup; Immediate window a stub | **Extended in I0 / I6** |
| Data science | `rapidr-value/src/datascience/` (RNum on `Vec<f64>`, RDataFrame rows of text cells with inferred column types, CSV / JSON readers, RPlot's model) — one implementation every runtime calls, the frame stored by columns; charts drawn by one renderer (`datascience/chart.rs` → the UI kernel's ops → `rapidr-ui-render`'s `chart.rs`), pixel-identical on desktop and web | One implementation since 2026-10-06 (D7), one chart renderer (L-FRAME); state in thread-local maps keyed by component name, no Parquet | **One implementation in I7** (engine decision D7) |
| Databases | `crates/rapidr-db` (1,027): RSQLITE (SQLite compiled in, also in wasm), RMYSQL (desktop), parameter binding | Imperative components only; no dataset / cursor model | **Extended in I7** (dataset model) |
| VS Code extension | `utilities/vscodeext/rapidr` (2,215 lines JS, no dependencies, MIT) | TextMate grammar, snippets, regex completion / hover / signature / outline, structural checks, build commands; no LSP, no debugger | **Switched to I3's LSP and I6's DAP** (thin client) |
| AI / MCP / LSP / DAP | — | None (plans only: ROADMAP Phases 4–5) | New in I3, I6, I8 |
| Accessibility | `objects/a11y.rs` (1,074), kernel `a11y.rs`, AccessKit 0.25 (desktop), ARIA mirror (`rapidr-ui-host-web/src/{aria,mirror}.rs`) | One tree, identical on both hosts (`tests/web_a11y.mjs`); probes `tools/macos/ax_dump.swift`, `tools/windows/uia_probe.ps1` | **Reused**; the editor gains text-run nodes (I2) |
| Testing | headless host captures (byte-identical desktop / web), `RAPIDR_TEST_*` hooks, `tests/gui_parity_cases.mjs` (83 cases), `tools/real_input.py`, `tools/regress.sh` | Proven for components | **Reused** for every IDE component (§6.4) |

What this means for the plan: the kernel, MDI, themes, accessibility and testing are ready to build on; the editor, designer, registry, parser-for-tools, debugger API and data stack each need real work; nothing exists yet for AI, MCP, LSP or DAP.

---

## 3. Architecture

### 3.1 Crates

```
rapidr-lang        NEW  the language registry: components (Q / R names, origin),       wasm-safe, no GUI
                        properties (type, default, editor, origin, docs), methods,
                        events, builtins, statements, directives, constants; generators
rapidr-project     NEW  the project file (.rrproj), workspace state, file kinds         wasm-safe
rapidr-session     NEW  the program session protocol (run / stop / debug / inspect /    wasm-safe
                        forms), both ends, transports (pipe, MessageChannel)
rapidr-editor      NEW  the editor's model: rope, selections, transactions, undo        wasm-safe, no GUI
                        tree, search, brackets, folding, indentation, the language
                        definition engine (declarative tokenizer)
rapidr-langsvc     NEW  the language service: semantic model on the parser, completion, wasm-safe
                        hover, signatures, navigation, rename, diagnostics, outline,
                        formatting, code actions; the Q / R compatibility checks
rapidr-lsp         NEW  `rapidr lsp` (stdio) over rapidr-langsvc                         desktop only
rapidr-dap         NEW  `rapidr dap` (stdio) over rapidr-session                          desktop only
rapidr-jsonrpc     NEW  JSON-RPC 2.0 framing shared by LSP and MCP                        wasm-safe
rapidr-ai          NEW  the provider layer (Anthropic, OpenAI-compatible, Gemini,         desktop: ureq + rustls;
                        Ollama, LM Studio), streaming, tool calls; RAI's engine           web: fetch
rapidr-secrets     NEW  API keys: OS keychain (desktop), session / opt-in encrypted (web)
rapidr-mcp         NEW  the MCP server: tool registry, permission tiers, audit log,       core wasm-safe
                        transports (stdio proxy over a local socket; loopback HTTP)
rapidr-frame       NEW  the one data-frame engine for every runtime: polars (D7); a   desktop: linked
                        separate wasm module on the web                                  web: lazy module
rapidr-value            + models: dock, editor view state, designer, inspector, tray,     shared models
                        data preview, plot (vector), dataset cursor
rapidr-ui-kernel        + components: dockmanager, codeeditor (virtualized),             one per model
                        formdesigner, inspector, projecttree, toolbox, console, …
rapidr-db               + the dataset model (cursor, fields, edit states)
ide/                NEW the IDE shell, a RapidR program (decision D3), replacing
                        examples/ide.rr; compiled into the release like today
```

Dependency rules (as the host plans): `rapidr-lang`, `rapidr-editor`, `rapidr-langsvc`, `rapidr-session`, `rapidr-project` and the MCP core never depend on a GUI or host crate, and build for `wasm32-unknown-unknown` (checked in `tools/regress.sh unit`, as the kernel is). Hosts never depend on the runtimes.

### 3.2 The IDE as a RapidR program on public components

The shell (menus, layout, wiring panels together, commands) is a RapidR program in `ide/` (several files joined by `$INCLUDE`), as `examples/ide.rr` is today, but written against the new components (decision **D3** — the recommendation, with its trade-offs, is in §9). Everything with performance or platform weight is Rust inside a component: the editor, the designer, the inspector, docking, the language service, the session, AI, MCP. The rule that keeps this honest: **the shell uses only public API**. If the shell needs something a component can't do, the component gains it (with docs and a test), never a private hook. So the IDE proves the components are complete, and users can build their own IDEs, database tools or editors from the same pieces.

The shell is compiled to bytecode for the web and (once the codegen covers it, measured on startup) to native code for the desktop release.

### 3.3 The program under development: its own process, its forms in the workspace

Today the web runs the program in a sandboxed iframe (`preview.html`, opaque origin, SEC-02 / SEC-03) and the desktop IDE shells out to `rapidr run`. The plan keeps that isolation and makes it one design:

- **Desktop**: the program runs in a child process (`rapidr run --session <pipe>`), the VM and the kernel's headless host inside it. **Web**: in a sandboxed opaque-origin iframe (as today), the program's VM in a worker there. Either way the program can't read the IDE's memory, keys or settings, a crash or a hang can't take the IDE down, and Stop is a kill.
- **One program session protocol** (`rapidr-session`, I0) between the IDE and the program, over a pipe (desktop) or a `MessageChannel` (web): start / stop / pause, breakpoints, stepping, stack, scopes and variables, evaluate, set property, output (stdout, ANSI), the forms (see below), the accessibility trees, files, test hooks. The DAP server (I6), the MCP tools (I8) and the IDE's own debugger views are all clients of it.
- **Forms as workspace windows ("remote forms")**: the program's kernel paints each shown form into a display list (the kernel's vector ops, images by content hash) and sends it with the form's accessibility tree, caret and IME rectangle; the IDE draws it inside an MDI child of its workspace (component `RProgramView`) and sends input back. Both hosts already render the same ops byte-identically, so the program looks exactly as it will standalone, at the IDE's scale. "Run detached" (the program's own top-level windows) stays a toggle (Run > Run in Separate Windows), needed anyway for programs that use the screen (full-screen DirectX, the tray). Remote forms arrive in I5 (they serve live preview and inspection); I1's Run uses separate windows on the desktop and the existing iframe on the web.

### 3.4 One codebase, two hosts

The IDE is the same bytecode on both: on the desktop `rapidr ide` runs it on the winit host; on the web the page loads the canvas host, the runtime and the IDE bytecode. Differences are capabilities, not code: the web has no native builds (needs the Rust toolchain; Build > Native is desktop-only), no local socket for MCP (§I8), and files through the File System Access API or the origin-private file system (OPFS) instead of the disk. These differences live in components (`RProject`'s storage, `RProgramSession`'s transport), not in the shell.

---

## 4. Stages

Each stage lists goals, components and APIs, data models, what it reuses or replaces, dependencies, size and lanes, and acceptance criteria. Public component APIs are summarized here and specified in [docs/ide-components.md](ide-components.md).

### I0 — Foundations

**Goals.** The things every later stage reads: one language registry, a project format, a program session protocol, and a parser that tools can trust.

**Components and APIs.**
- `rapidr-lang`: the registry. Source: data files in the repo (`crates/rapidr-lang/data/*.toml`, one per component family; decision **D5**) compiled into static tables by `build.rs`. Generators: `rapidr lang export --json` (the IDE's completion data, VS Code's data), `--manual` (the manual's reference sections), `--prompt` (the AI system prompt's language section), `--rust` checks. Tests tie it to reality: every component / property / method / event in the registry is answered by both runtimes (a generated conformance program per component, run native, interpreted and on the web), and every name the runtimes answer is in the registry. `rapidr-ast::COMPONENT_TYPES`, the duplicated `is_component_type` / `is_component_method` lists in `runtime-core/object.rs` and `runtime-web/object_web.rs`, and `component_defaults`' defaults are generated from it (one integrator, §7).
- Origin data for the Q / R rules ([docs/q-and-r-components.md](q-and-r-components.md)): each component's RapidQ name (or none), each property / method / event / builtin / statement / directive marked RapidQ or RapidR, from RapidQ's manual, `KEYWORD.LST` and RC.EXE probes (facts only; every description in the registry is written in our own words, not copied from RapidQ's help).
- `rapidr-project`: the project file (decision **D2**; recommended TOML, `.rrproj`), loading the web IDE's JSON v1 (`rapidr_project: 1`: forms as JSON → CREATE blocks in `.rr` files, inline data-URL assets → files), and plain `.bas` / `.rr` files opened without a project (an implicit one, RapidQ's way).
- `rapidr-session`: the protocol (§3.3) and its two ends — the program side in the VM hosts (`rapidr-vm-host-native`, `rapidr-vm-host-web`), the IDE side as the `RProgramSession` component. Today's `DebugSession` wasm API and the `__rapidr_debug_*` messages become one transport of it.
- VM debugging primitives the protocol needs (in `rapidr-vm`): per-file breakpoints through the source map (and the web compile path filling `source_map` like the CLI does), pause on demand at the next safe point, an evaluation entry point (compile a snippet against a frame's symbols: bcgen exposes the frame's scope), writing a variable, a "break on runtime error" flag. Conditions, hit counts and logpoints come in I6 on top.
- Parser for tools (in `rapidr-lexer` / `rapidr-parser` / `rapidr-preprocessor`): comments, blank lines and directives kept as **trivia** in a side table (the AST stays as it is for the backends); a byte-level origin map from preprocessed text to (file, byte offset) — not just lines — so spans point into the files the user edits; the lexer recovers (an error token, then on) instead of aborting; a public semantic model out of bcgen's private `Scope` / `Bcgen` tables (declarations, their spans and types, references) for I3. **No change to what any program means** — checked by the conformance suite, the corpus comparison and the web parity suite.

**Data models.**
- Registry: `Component { name: "RBUTTON", rapidq_name: Some("QBUTTON"), origin, family: RapidQ | RapidR, group, icon, visual, container, non_visual_tray, default_size, properties: [Property { name, ty: Int | Float | Str | Bool | Color | Font | Enum{values} | Set{flags} | ComponentRef{kinds} | Picture | Strings | Columns | Sql | Expression | File{filters}, default, origin, design_time, run_time_only, read_only, category, editor, doc, since }], methods: [Method { name, params: [Param { name, ty, optional, default, byref }], returns, value_method, doc, origin }], events: [Event { name, params, default_event, doc, origin }] }`; `Builtin { name, signature, returns, doc, origin, bare }`; `Statement`, `Directive`, `Constant`, `Keyword`.
- Project: `Project { format: 2, name, main, files: [ProjectFile { path, kind: Module | Form | Include | Resource | Asset | Data }], build: { apptype, icon, theme, targets, release }, run: { args, cwd, separate_windows }, compat: { rapidq_compatible: bool, level: warn | error }, designer: { grid, snap, guides }, ai: { allowed, context_include, context_exclude, data_access: none | schema | sample | full, sample_rows, redact_columns } }`. Per-user state (open documents, dock layout, breakpoints) in `.rapidr/workspace.toml` beside it, not shared (ignored by VCS).
- Session messages: `Start { program, args, debug }`, `Stop`, `Pause`, `SetBreakpoints { file, lines: [Bp { line, condition, hit, log }] }`, `Continue | StepIn | StepOver | StepOut`, `Stopped { reason, file, line }`, `StackTrace`, `Scopes`, `Variables { ref }`, `Evaluate { expr, frame }`, `SetVariable`, `SetProperty { object, prop, value }`, `Output { stream, text }`, `FormShown / FormFrame { id, ops, a11y, caret } / FormClosed`, `Input { form, event }`, `Exited { code }`.

**Reuses / replaces.** Reuses `COMPONENT_TYPES`, `builtins::BUILTINS`, `component_defaults`, `canonical_type_name`, the VM's debug state, `DebugSession`, the preprocessor's line map. Replaces both `lang-data.js` files (generated, then deleted with their hosts), the hand tables in `host.js` (`EVENT_META`, `COLOR_PROPS` …), the toolbox lists in `examples/ide.rr`.

**Depends on.** Nothing. Starts now; the parser work coordinates with any running compatibility lane (one owner of `rapidr-lexer` / `rapidr-parser`).

**Size.** L, 12–16 sessions, 4 lanes: registry (L-REG), project + session (L-SESS), VM debugging primitives (L-VM), parser for tools (L-PARSE).

**Acceptance.**
- The registry covers every component in `COMPONENT_TYPES`, every builtin in `BUILTINS`, every statement and directive the parser accepts; the both-runtimes conformance test and the reverse check pass; `rapidr-ast`'s and the runtimes' name lists are generated and the old hand lists are gone.
- Generated VS Code data replaces `languageData.js` (the extension unchanged otherwise); `lang-data.js` generated until `web-ide/` goes.
- Every RapidQ example of the corpus parses with trivia, and printing trivia + tokens reproduces each file byte for byte; spans of a program with `$INCLUDE`s point into the right file at the right byte.
- `rapidr run --session` and the web iframe speak the protocol; `tests/web_ide_debugger_test.mjs` and the debug e2e suites pass through it; a breakpoint in an included file stops on both runtimes.
- Conformance, corpus comparison, web parity: unchanged results.

### I1 — Shell

**Goals.** A good-looking, easy, modern IDE: an MDI workspace with docking panels, project tree, toolbox, property inspector, modern light / dark / high-contrast themes, commands and keyboard shortcuts, run / stop through the interpreter; the same IDE on the desktop and the web; the HTML IDE deleted.

**Components and APIs** (public; [docs/ide-components.md](ide-components.md)).
- **RDockManager**: panels docked left / right / top / bottom, split, tabbed, auto-hide, floating (a top-level window on the desktop, a floating kernel window on the web), dragged with a docking compass, all keyboard-operable (F6 cycles areas, a "Move panel" command), layouts saved / restored by name (`SaveLayout` / `LoadLayout` as text). Its **document area** is an MDI client (`rapidr_value::mdi`: cascade, tile, window list) or tabbed documents (a per-user choice; MDI the default, the user's direction).
- **RProjectTree**: the project's files by kind, forms with their components, rename / move / delete with confirmation, drag to reorder, file-system watching (desktop), OPFS / File System Access (web).
- **RToolbox**: the registry's components grouped under **"RapidQ"** (Standard, Additional, Dialogs, System, Media …) and **"RapidR"** (Data, Data Science, Web, AI, IDE), each showing the name the designer will write (QBUTTON, RPLOT); search; drag to the designer or double-click to add; user component templates.
- **RPropertyInspector**: Delphi's object inspector — categories or A–Z, search, typed editors from the registry (number, text, Boolean, enum, set, colour, font, picture / asset, strings list, columns, component reference picker, SQL, file), RapidR-only properties of RapidQ components grouped under "RapidR extensions" with a badge, an Events tab (double-click → handler stub, or pick an existing SUB with a matching signature), multi-selection editing (common properties).
- **ROutputConsole**: the program's output with ANSI (`web-ide/ansi_screen.js`'s screen model moved into Rust), the build log, problems (clickable diagnostics), search.
- **RCommandPalette**, **RToolBar** (a real kernel kind: icon buttons, separators, overflow, tooltips, customizable), **RStatusBar** sections (caret, encoding, compat mode, AI state).
- **RProject** and **RProgramSession** (non-visual): open / save / new from template; Run (F5), Run Without Debugging (Ctrl+F5), Stop (Shift+F5), Build (native / interpreted executable, `.rrbc`, web bundle).
- IDE chrome: the modern theme's tokens extended for the IDE (panel headers, tabs, docking hints, editor token colours per theme), vector icons (a permissive set — Lucide, ISC, or Tabler, MIT — plus our own component icons; ROADMAP's "IDE icons as vectors"), the OFL UI and editor fonts (decision **D8**), settings (a searchable page; JSON-free UI), keyboard shortcut scheme (VB6 / Delphi defaults: F5, F8 / F7, F9, F11 / F12; a VS Code scheme option).
- Web specifics: the page (`ide/web/index.html` + the runtime), autosave to IndexedDB / OPFS (ROADMAP's item), File System Access open / save where available, zip import / export elsewhere, the web bundle build (the existing `zip.js` logic moved into Rust, as `rapidr-webbundle` already does natively).

**Data models.** `DockLayout = Node`, `Node::Split { axis, sizes: [f32], children } | Node::Tabs { panes: [PaneId], active } | Node::Documents (the MDI area) | Node::AutoHide { side, panes }` plus `floating: [{ rect, node }]`; `Pane { id, title, icon, component, closable }`. `Command { id, title, category, shortcut, enabled_when }` in a table the menus, toolbar, palette and keyboard share.

**Reuses / replaces.** Reuses `rapidr_value::mdi` and its frame drawing (generalized as the web host plan's `WindowFrame`), the tree / grid / list view / tab / menu / splitter / status bar kernel components, the themes, `rapidr-webbundle`, the CLI's build steps. Replaces `examples/ide.rr` and `web-ide/` (after acceptance), the web IDE's `.rrproj` v1 (read for migration), ROADMAP Phase 3's "split host.js" and "Tauri desktop shell" items (no longer needed: the kernel host is the desktop shell).

**Depends on.** I0 (registry for toolbox and inspector, project, session for run / stop). I2–I4 plug into its document area as they land; until then the I1 shell hosts the current RCODEEDITOR and RDESIGNSURFACE.

**Size.** L, 16–22 sessions, 4 lanes: docking + workspace (L-DOCK), inspector + toolbox + tree + toolbar (L-PANELS), shell assembly + commands + settings + themes / icons (L-SHELL), web page + storage + run (L-WEB).

**Acceptance.**
- The parity checklist: every feature of `web-ide/` (its menus, the 27 `tests/web_ide_*.mjs` suites re-pointed at the new IDE, assets manager, build zip, undo, themes, examples list) and of `examples/ide.rr` exists in the new IDE on both hosts, or is listed as deliberately dropped with a reason. **Then `web-ide/`, Monaco, `examples/ide.rr`, `examples/web_ide.rr` and the DOM-IDE tests are deleted in the same commit**; `rapidr ide` and the web IDE URL open the new IDE.
- Docking: every layout operation by mouse and by keyboard; a layout round-trips through `SaveLayout` / `LoadLayout`; floating panels on the desktop are real windows; the same layout drawn byte-identically on both hosts (captures).
- Accessibility: VoiceOver (macOS), NVDA (Windows) and Chrome's tree reach every panel, toolbar button, tree node, inspector row and menu with names and roles; nothing needs a mouse.
- Performance (§6.2): cold start and project open within targets on the reference machines.
- Run / Stop works on both hosts for every program in `examples/` and the portable corpus' GUI programs.

### I2 — Code editor

**Goals.** Our own editor on the kernel, at the level of a modern code editor: rope buffer, syntax colouring, folding, multi-cursor, find / replace, bracket matching, auto-indent, large files, IME, accessibility; language-agnostic through declarative language definitions.

**Components and APIs.** **RCodeEditor** (the existing RCODEEDITOR grown; every current property, method and event keeps answering as today — Text, Lines, LineCount, SelStart / SelLength / SelText, WhereX / WhereY, GetSubList, GotoSub, GotoLine 0-based, OnChange). New: `Language`, `ColorScheme`, `TabSize`, `InsertSpaces`, `WordWrap`, `ShowLineNumbers`, `ShowFolding`, `ShowMinimap`, `ShowWhitespace`, `ReadOnly`, `CaretLine` / `CaretColumn` (1-based, new), `CursorCount`, `Modified`, `CanUndo` / `CanRedo`; methods `Undo`, `Redo`, `Find`, `FindNext`, `Replace`, `ReplaceAll`, `AddCursor`, `SelectNextOccurrence`, `Fold` / `Unfold` / `FoldAll`, `InsertText`, `ApplyEdits` (a JSON list of range edits, one undo step), `SetDiagnostics`, `AddMarker`, `ShowCompletion` / `ShowHover` / `ShowSignature` (the UI that I3 feeds and that users' programs can feed), `LoadFromFile` / `SaveToFile`, `BeginUpdate` / `EndUpdate`; events `OnCaretMove`, `OnSelectionChange`, `OnGutterClick`, `OnCompletionRequest`, `OnHoverRequest`, `OnSignatureRequest`, `OnSave`. Plus **RDiffView** (side-by-side / inline diff on the same view, accept / reject per hunk; used by I8 and by "compare with saved").

**Model** (`rapidr-editor`, GUI-free):
- **Buffer**: a rope (decision **D6**: ropey 1.6, MIT — stable, used by Helix — behind our own `Buffer` trait; ropey 2 / crop as byte-indexed alternatives later), byte offsets internally, UTF-16 columns computed for LSP / web, line endings preserved per file (CR LF for `.bas` written by RapidQ tools).
- **Selections**: `Vec<Selection { anchor, head, goal_column }>`, merged when they overlap; every edit is a `Transaction { changes: [Change { range, insert }], selections_before, selections_after }`.
- **Undo tree** (linear undo / redo by default, history kept), typing grouped by word / pause (400 ms), one step per command or `ApplyEdits`.
- **Search**: literal / whole word / case / regex (the `regex` crate, MIT / Apache), incremental, in selection, replace with groups, across files (I1's Find in Files on the project).
- **Language definitions** (declarative, [docs/ide-components.md](ide-components.md) §2): comments, strings (escapes, RapidQ's `$ESCAPECHARS`), numbers, keyword groups (case-insensitive for BASIC), operators, brackets and auto-closing pairs, indentation rules, folding (by markers such as `SUB … END SUB`, or by indentation), word pattern, snippets; a small state machine tokenizer with regex rules and a state carried line to line (the current `objects::code` contract), so re-colouring after an edit stops as soon as a line's end state is unchanged. Built in: RapidQ / RapidR BASIC (generated from the registry's keywords), plain text, JSON, SQL, CSV, Markdown, HTML, CSS, JavaScript, TOML, Rust (for `RUSTSTART` blocks: an embedded-language region). **No tree-sitter**: its C runtime complicates the `wasm32-unknown-unknown` build, there is no RapidQ grammar, and BASIC's real structure comes from our own parser through I3's semantic tokens.
- **Folding, brackets, indentation, whitespace and the minimap** computed from the tokens and I3's outline.

**View** (kernel component `codeeditor.rs`, replacing `codeedit.rs` on the memo):
- **Virtualized**: only visible lines (plus a margin) have parley layouts, in an LRU cache keyed by (line content hash, style revision); line heights are fixed per font (word wrap measures wrapped lines lazily), so scroll position → line is arithmetic and a 1-million-line file costs the same per frame as a 100-line one.
- Gutter (numbers, breakpoints, markers, fold arrows, diff marks), squiggles, inlay hints, the current line, bracket pairs, rulers, multiple carets, selections, the completion / hover / signature popups in the kernel's popup layer (above the web host's overlays, as menus).
- Colour schemes per theme: token kinds → colours in the theme table (fixes today's dark-theme contrast).
- **IME**: the kernel's preedit / commit path (desktop) and the mirror's text field (web), placed at the caret (`ime_area`), per caret 0.
- **Accessibility**: a multiline text node with **AccessKit text runs** for the visible lines plus the caret's line (so screen readers read by character, word, line), the selection, line / column announced on move, diagnostics announced on the caret's line (a live region), completion as a list box with the active item; on the web the mirror's `<textarea>` holds a window of lines around the caret (VS Code's approach), with an "Accessible view" command (the whole document as plain text) for long reads.

**Reuses / replaces.** Reuses parley and the kernel's `text/editor.rs` paragraph layout and styled runs (`Span`, `RunStyle`), `objects::code`'s contract, the memo's scroll bars and clipboard paths, the IME paths of both hosts. Replaces `TextEdit::code()`'s `Vec<char>` buffer for RCODEEDITOR (QMEMO / QRICHEDIT keep theirs), the memo-based `codeedit.rs`, Monaco (with `web-ide/`).

**Depends on.** I0 (keywords for BASIC from the registry), I1 (document area; can be developed standalone in a test form before).

**Size.** L, 14–20 sessions, 3 lanes: model (L-EDCORE), view + popups + diff view (L-EDVIEW), IME + accessibility + performance (L-EDA11Y).

**Acceptance.**
- Typing latency, open time, scrolling and memory targets (§6.2) met on a 200,000-line / 10 MB file on both hosts; a benchmark harness in `tools/regress.sh perf`.
- Property tests: random transactions + undo / redo restore the exact text and selections; multi-cursor edits equal sequential single edits; the tokenizer's incremental result equals a full re-tokenize.
- Every existing RCODEEDITOR fixture (`code_editor` case) unchanged in dumps on native, interpreted and web.
- IME: Japanese, Chinese and Korean input in the editor on macOS, Windows, Linux and Chrome / Firefox / Safari (by hand, §6.4); dead keys and emoji pickers.
- Screen readers read and edit code by character / word / line, hear line and column, diagnostics and the completion list (VoiceOver, NVDA, Chrome's tree).

### I3 — Language service

**Goals.** IntelliSense that knows the program: completion of locals, globals, SUBs / FUNCTIONs, TYPEs, components and their members (by the variable's type, including `WITH` and inside `CREATE`), hover with docs, signature help, go to definition, find references, rename, live diagnostics in RapidQ's compiler wording, outline, code actions and formatting. One Rust crate, built native (desktop IDE, LSP) and to wasm (web IDE), so both IDEs and VS Code get the same answers.

**Components and APIs.** `rapidr-langsvc` (Rust API: `Analysis::new(project) → update(file, edits) → completions(pos) / hover(pos) / signature(pos) / definition(pos) / references(pos) / rename(pos, new) / diagnostics(file) / outline(file) / semantic_tokens(file) / format(file) / code_actions(range)`). Exposed as the non-visual **RLanguageService** component (users can drive it, e.g. a teaching tool), wired to RCodeEditor's request events by the shell. **`rapidr lsp`** (stdio; `lsp-server` MIT / Apache, `lsp-types` MIT; synchronous, no async runtime) with completion, hover, signature help, definition, references, rename, document symbols, semantic tokens, diagnostics, formatting, code actions; the VS Code extension becomes an LSP client (its regex providers deleted).

**Data models.** `SemanticModel { files, symbols: [Symbol { name, kind: Local | Global | Param | Sub | Function | Type | Field | Component | Constant | Label, ty, decl: Span, scope }], references: [(Span, SymbolId)], scopes }`, built from I0's parser output and bcgen's scope rules (RapidQ's implicit scope, `$TYPECHECK`, `$OPTION EXPLICIT`, `DEFINT` and suffixes, case-insensitivity, include files). Types come from the registry for components (Q and R names alike: one model), from `TYPE` declarations, suffixes and `DIM … AS`. Incrementality: per-file re-parse (files are small; a 20,000-line file re-parses in well under the budget), project-level symbol index updated per file; requests answered from the last good model while a re-parse runs.

**Q / R behaviour.** Completion after `AS` offers RapidQ's components under Q names and RapidR-only ones under R names (the designer's rule); hover shows both names and the origin; members of a QBUTTON and an RBUTTON are the same list, RapidR-only members marked. **RapidQ-compatible projects** get a diagnostic for every RapidR-only component, property, method, event, builtin, statement or directive, and for Q-prefixed names RapidQ doesn't have (`QPLOT`), with code actions ([docs/q-and-r-components.md](q-and-r-components.md) §5).

**Automatic case** (QuickBASIC's and VB's): `Analysis::case_edits(file, CaseScope::Typed { offset, ch } | Range { start, end })` puts the registry's words (keywords, statements, types, directives, builtins) in the `keywordCase` asked (upper — the default —, lower, proper, preserve) and, with `identifierCase = declaration`, the program's names as declared and components' members as the registry spells them; strings, comments, directives' arguments and the program's own names are never touched (the lossless tokens and the semantic model decide, no regexes). `rapidr lsp` serves it as on-type formatting and in document / range formatting (`format` = indentation + case, non-overlapping edits). **RapidR Studio's editor (`crates/rapidr-editor`) calls the same function in wave B**, after each trigger character of `case::TRIGGERS` and in Format Document.

**Reuses / replaces.** Reuses the lexer, parser, preprocessor, bcgen's scope rules (moved into the semantic model, with bcgen reading them back so the compiler and the IDE can't disagree), the registry, RapidQ's compiler messages (`.reference/rapidq-compiler-messages.txt`). Replaces `monaco-host.js`'s providers, `resolveVariableType`, the VS Code extension's `completionProvider.js`, `hoverProvider.js`, `signatureProvider.js`, `symbolProvider.js`, `typeParser.js` and its regex validation, and the string-matched `line:col: error:` diagnostics path (structured diagnostics instead).

**Depends on.** I0 (registry, parser for tools, semantic model export). The UI half needs I2's popups.

**Size.** L, 14–20 sessions, 3 lanes: semantic model + diagnostics (L-LS), completion / hover / signature / navigation / rename / formatting (L-LSFEAT), LSP + VS Code client + the editor UI wiring (L-LSP).

**Acceptance.**
- Completion latency < 50 ms (p95) in a 20,000-line project, native and wasm; diagnostics < 300 ms after typing stops.
- A golden test suite (`tests/langsvc/`): for each case a source with markers (`|`) and the expected completion list / hover / definition / references; includes `WITH`, `CREATE` bodies, TYPE methods, include files, Q and R names, RapidQ's implicit scope.
- Rename across files is exact (references from the semantic model, never text search) and refuses on conflicts.
- Diagnostics equal the compiler's (same messages, same spans) for every conformance error case (`*.expected-error`).
- Every corpus program and `examples/` file analyses without a panic (fuzzed: random edits on corpus files for an hour, no panic, no hang).
- The VS Code extension works through `rapidr lsp` with the same results as the IDE.

### I4 — Visual designer

**Goals.** Drag and drop like Xcode / Xojo / Delphi: the real components drawn WYSIWYG, grid snapping, smart guides with centring, multi-select, align and distribute, anchors, a tab-order editor, a menu editor, undo / redo, and two-way sync between designer and code (CREATE blocks), keeping user code intact.

**Components and APIs.** **RFormDesigner** (new; RDESIGNSURFACE's methods and events answered by the same model for existing programs): properties `Form` (the designed form's source), `GridSize` (8), `ShowGrid`, `SnapToGrid`, `SnapToGuides`, `ShowGuides`, `SelCount`, `Zoom`, `PreviewTheme`, `PreviewScale` (1×, 1.5×, 2×); methods `AddComponent`, `Delete`, `SelectAll`, `Align(how)`, `Distribute(how)`, `SameSize(how)`, `BringToFront` / `SendToBack`, `Nudge(dx, dy)`, `Undo` / `Redo`, `Cut` / `Copy` / `Paste`, `SetProperty(names, prop, value)`, `GetSelected`; events `OnSelectionChange`, `OnPropertyChange`, `OnComponentAdded` / `Removed`, `OnDblClick(Name)` (default event → handler), `OnContextMenu`. Plus **RComponentTray** (non-visual components under the form: timers, dialogs, databases, I7's data components), **RTabOrderEditor**, **RMenuEditor** (QMAINMENU / QPOPUPMENU trees: captions, `&` mnemonics, shortcuts, checked / enabled, separators, OnClick handlers).

**Behaviour.**
- **WYSIWYG**: each designed component is the real kernel component painted from a design-time store (the kernel's `MemStore`), so a QLISTVIEW looks like a QLISTVIEW, at the chosen theme and scale; input goes to the designer, not the component (a design-mode flag the kernel already approximates for RDESIGNSURFACE).
- **Snapping and guides** (the designer model, pure functions, unit-tested): snap to the grid (8 px, configurable; RapidQ pixels), to sibling edges, centres and **baselines** (a label's text baseline to an edit's), to the parent's centre lines (centring), to **margins** (8 px from the parent's edges and between siblings, the platform spacing), and **equal spacing** (when the gap to one neighbour equals another gap in the row / column, show the distances). Guides are drawn as thin lines with distance labels; Alt (Option) suspends snapping; the snap threshold is 5 logical pixels at any zoom.
- **Selection**: click, Shift / Ctrl-click, rubber band, Tab / Shift+Tab through components, Esc to the parent; drag moves, eight handles resize, arrows nudge 1 px, Shift+arrows by the grid, Ctrl / Cmd+arrows resize; copy / paste keeps names unique (Button1 → Button2) and handlers unbound.
- **Align / distribute / same size**: left, centre, right, top, middle, bottom (to the first-selected, to the parent), distribute horizontally / vertically, same width / height / both, centre in parent.
- **Anchors and constraints** (RapidR's `Anchors` / `Constraints`, already in `rapidr_value::layout`): an anchor editor in the inspector and on-canvas anchor pins; a resize preview of the form shows how anchored components follow.
- **Containers**: drop into panels / group boxes / tab pages / scroll boxes (reparenting = `Parent` and the CREATE nesting), with a highlight of the target.
- **Undo / redo**: every designer change is a command (`Move`, `Resize`, `SetProperty`, `Add`, `Remove`, `Reparent`, `Rename`, `ZOrder`, `TabOrder`, `Menu*`) whose effect is a text transaction on the form's source — so the editor's undo and the designer's are **one history** per file.
- **Events**: double-click → the default event's handler (created with the registry's parameter list, or jumped to); the inspector's Events tab lists events with existing compatible SUBs.

**Two-way sync** (the hard part; the designer never owns a separate copy of the form):
- **The source is the truth.** A form is the `CREATE Name AS Type … END CREATE` block (nested CREATEs for children) found by the parser's spans (I0); there are no designer files and no marker comments, so RapidQ programs from anywhere open as they are.
- **Designer → code**: each command becomes the **smallest text edit**: change a property's value span only; add a property line after the block's last property line with the block's indentation; remove only lines that are a single property assignment; add a child as a new nested CREATE before the parent's `END CREATE`; rename updates the CREATE, the handler wiring (`OnClick = Button1Click`) and — through I3's rename — every reference. Comments, blank lines, statements inside CREATE that aren't property assignments (`Center`, calls, `IF`s), and everything outside the blocks stay byte-identical.
- **Code → designer**: an edit re-parses the file (debounced, ~150 ms) and the designer updates. While the file doesn't parse, the designer keeps the last good state with a banner ("code has errors at line N") and is read-only.
- **What the designer shows but doesn't own**: properties set by code outside the CREATE (e.g. in the form's OnShow) are shown greyed with "set in code, line N"; statements inside CREATE that the designer can't represent are listed under the component as "code in CREATE" (jump to them).
- **Forms not in CREATE blocks** (`DIM f AS QFORM` + property assignments): opened read-only with an offer to convert (a reviewed text edit).
- **Names** ([docs/q-and-r-components.md](q-and-r-components.md)): new RapidQ components are written with their Q names (QBUTTON), R-only ones with R names (RPLOT); a component already in the source keeps the name it was written with. Property values are written in RapidQ's spelling (colours as `&H` BGR numbers or RapidQ constants, fonts as RapidQ does).

**Data models.** `FormDesign { root: NodeId, nodes: Arena<DesignNode { type_name_as_written, canonical, name, props: [(name, value_text, span)], children, span, extra_code: [Span] }>, tray, selection: [NodeId], guides: [Guide { axis, at, from, to, kind: Edge | Centre | Baseline | Margin | Spacing(px) }] }`; `DesignCommand` as above; `TextPatch { range, insert }` produced from commands.

**Reuses / replaces.** Reuses `objects::design`'s hit testing and grips (generalized to eight handles and multiple selection), the kernel components themselves for drawing, `rapidr_value::layout` (Anchors / Constraints), the menu model (`objects::menu`), the web IDE's align / size / z-order command set (reimplemented in Rust), I3's rename. Replaces RDESIGNSURFACE's placeholder drawing, `examples/ide.rr`'s `GenerateCode` / `SyncEventCodeFromSource` / `ExtractSub` string surgery, and the web IDE's JSON-forms-plus-generated-code model.

**Depends on.** I0 (parser spans with trivia, registry), I1 (inspector, toolbox, tray host), I3 (rename, handler signatures; the sync can start before I3 with I0's spans).

**Size.** L, 16–22 sessions, 4 lanes: designer model + guides (L-DMODEL), designer view + WYSIWYG + tray + menu / tab-order editors (L-DVIEW), two-way sync (L-SYNC), inspector integration + events (with L-PANELS).

**Acceptance.**
- **Anchoring is a first-release must (the user, 2026-10-06)**: anchor pins on the selected component's four sides (click to toggle, keyboard too), the inspector's Anchors / Constraints / Align editors, and a resize preview (drag the designed form's corner: anchored components follow exactly as at run time, on both hosts); the designer writes `Anchors = …` / `Align = …` into the CREATE block as the smallest edit; a scripted test resizes a designed form and compares each component's bounds with the running program's.
- **Round trip on the corpus**: every form of the 386 RapidQ examples and of `examples/` opens in the designer; saving without a change leaves every file byte-identical; a scripted edit (move one component 8 px) changes exactly one line's value per changed property, and the program still compiles and runs identically apart from that change (checked by the corpus comparison tool).
- Property-based test: random designer commands applied, then the text re-parsed into the designer, equals the designer's state (both directions agree); undo restores the exact bytes.
- Guides: unit tests for each guide kind and the snapping math; 60 frames per second while dragging on a form with 300 components (guide computation < 2 ms per move).
- Everything by keyboard: select, move, resize, align, add from the toolbox (Enter on a toolbox item adds at the selection's container), with screen-reader announcements of name, type, position and size.
- Captures: the designed form equals the running form pixel for pixel (same theme and scale), on both hosts.

### I5 — Live

**Goals.** See changes without restarting: live property preview into the running program, hot reload of code, edit-and-continue while paused; the program's forms inside the workspace (remote forms, §3.3) with "inspect element".

**Components and APIs.** **RProgramView** (an MDI child showing a remote form: draws the program's display lists, forwards input, mirrors its accessibility tree); session messages `SetProperty`, `ReloadModule`, `ReplaceFunction`, `SetNextStatement`; `RProgramSession.HotReload` (Boolean), `OnReloaded(Report)`.

**Behaviour.**
- **Live property preview**: while the program runs, a designer change to a component that exists in the running program (same form and name) is also sent as `SetProperty` — the form changes on screen at once; the source edit is what persists.
- **Hot reload** (running, not paused): on save, the module is recompiled; functions whose signature and locals' layout didn't change get their new bytecode (frames already inside an old version finish it); new SUBs / FUNCTIONs and new globals are added; anything else (a TYPE's layout, a global's type, CREATE blocks' structure) reports "restart needed" with the reason. Property values in changed CREATE blocks are applied as live property sets. Semantics: what a restart would do for the changed code, nothing more (rule 1).
- **Edit-and-continue** (paused): the same, plus the current function if the edit keeps its locals' layout, remapping the instruction pointer to the same statement (by line table); otherwise Set Next Statement or restart.
- **Inspect element**: click a control in a remote form (with the inspect tool) → the IDE selects that component in the designer and the inspector shows its live properties; hover shows its handlers.

**Reuses.** The VM's function table and line tables, I0's session and evaluation, the kernel's display lists, the web host's and the desktop host's rendering. Native builds are never hot-reloaded (debug on the VM, ship on either backend: ROADMAP Phase 2's policy).

**Depends on.** I0, I4 (designer), I6's basics (pause / stack).

**Size.** M, 8–12 sessions, 2 lanes: remote forms + inspect (L-REMOTE), hot reload / edit-and-continue (L-VM).

**Acceptance.** A scripted suite: change a handler's body while the program runs → the next click runs the new code (both hosts); change a CREATE property → the running form updates within 100 ms; an incompatible edit reports "restart needed" with the reason; E&C inside a paused SUB continues with the new line. Remote forms equal the standalone program's captures byte for byte; screen readers reach the program's controls inside the IDE.

### I6 — Debugger

**Goals.** A debugger as good as VB6's and Delphi's, on the interpreter: breakpoints (conditional, hit counts, logpoints, break on runtime error), watches and data tips evaluated by the VM, call stack with frames' locals, stepping, the immediate window; DAP for other editors.

**Components and APIs.** **RBreakpointList**, **RCallStackView**, **RVariablesView** (locals / globals / watches tree; objects expand into properties; arrays page by 100; frames open in RDataPreview), **RImmediateWindow** (`? expr` prints, statements run in the paused frame, history), data tips in RCodeEditor (hover a variable while paused), the gutter's breakpoint glyphs (conditions edited inline). **`rapidr dap`** (stdio) maps DAP onto the session protocol; the VS Code extension registers it.

**Behaviour.** Conditions and log messages are compiled with the evaluator (I0) against the breakpoint's scope; hit counts (=, ≥, multiple of); logpoints print `{expr}` interpolations without stopping; break on runtime error stops at the faulting statement with the error; pause (break all) at the next safe point; while paused, the program's timers and events don't run (queued; the web host's paused-overlay logic moves into the session), and the IDE shows the program's forms frozen. Watches update at each stop; evaluation has a fuel limit (no hanging the IDE with `WHILE 1: WEND` in a watch). Later (ROADMAP Phase 2, after I6): reverse stepping, the event timeline, the line profiler.

**Reuses / replaces.** Reuses the VM's breakpoints and stepping, I0's evaluator and session. Replaces the web IDE's debugger panels and its JS `evaluateWatchExpression`.

**Depends on.** I0; I2 (gutter, data tips); I1 (panels).

**Size.** M, 8–12 sessions, 2–3 lanes: VM conditions / logpoints / errors (L-VM), views + immediate window (L-DBGUI), DAP (L-LSP). **Basic debugging for the first release** (4–6 sessions): line breakpoints in any file, conditions, step in / over / out, pause, run to cursor, call stack, locals / globals / watches evaluated by the VM, data tips, break on runtime error, the immediate window.

**Acceptance.** `tests/web_ide_debugger_test.mjs` and the debug e2e suites (re-pointed), plus new cases: a conditional breakpoint inside an event handler stops only when true; a logpoint prints and doesn't stop; break on error shows the right file and line in an `$INCLUDE`d file; a watch with an infinite loop is stopped by fuel; same results native-hosted (desktop child process) and web. VS Code debugs a program through `rapidr dap` (breakpoint, step, variables).

### I7 — Linked data and data science

**Goals.** Delphi / Lazarus-style linked components, with the data-science stack first-class: a non-visual component tray; files and connections → datasets / data frames → transform chains → a data source → data-aware controls and RPlot charts; component-reference properties edited with pickers; **live data at design time** (point a CSV at a chart and see the plot in the designer, updating as data or settings change); a data preview panel. All are additive R-prefixed components: RapidQ programs are untouched (rule 1).

**Components** (APIs in [docs/ide-components.md](ide-components.md) §4).
- **Sources**: `RDBConnection` (Driver = sqlite | mysql, Database, Host, Port, User; the password from the keychain at design time, from the program at run time, never written into source), `RDataFile` (FileName, Format = auto | csv | json | ndjson | parquet, Delimiter, HasHeader, Encoding, Watch), `RDBQuery` (Connection, SQL, Params, Active), `RDBTable` (Connection, TableName, Filter, OrderBy).
- **The frame**: the existing `RDataFrame` gains `Source` (any source or transform) and stays usable imperatively (its current methods unchanged).
- **Transforms** (each one is a frame and can be the `Input` of the next, chained in the designer like Delphi datasets): `RDFFilter` (Condition), `RDFSort` (Columns, Descending), `RDFGroup` (GroupBy, Aggregates such as `"Sales:sum, Qty:mean"`), `RDFJoin` (Left, Right, On, How), `RDFCompute` (Column, Expression), `RDFSelect` (Columns, Rename), `RDFLimit` (Rows, Offset). Conditions and expressions are **BASIC expressions over column names** (`Sales > 100 AND Region = "EU"`, `FORMAT$(Date, "yyyy-mm")`), parsed by our parser and compiled to the engine's expressions — no SQL or second expression language to learn, and no code injection (it's an expression tree, not text evaluated).
- **The binding hub**: `RDataSource` (DataSet = any dataset or frame, AutoEdit, Enabled; OnDataChange, OnStateChange) — Delphi's TDataSource.
- **Data-aware controls**: `RDBGrid` (DataSource, Columns editor; virtual rows: millions of rows scroll at frame rate), `RDBEdit` / `RDBLabel` / `RDBMemo` / `RDBCheckBox` / `RDBComboBox` (DataSource, DataField), `RDBLookupCombo`, `RDBNavigator` (First / Prior / Next / Last / Insert / Delete / Edit / Post / Cancel / Refresh, each enabled by the dataset's state).
- **Output**: `RPlot` gains declarative binding — `DataSource` (or `Frame`), `Kind` (line, bar, barh, scatter, area, step, hist, pie), `X`, `Y` (one or several columns), `Series` / `ColorBy`, `Title`, `XLabel`, `YLabel`, `Legend`, `Stacked` — beside its existing imperative methods (unchanged).
- **IDE**: the **component tray** (I4) shows them with their links drawn as lines on demand; **component-reference pickers** in the inspector (a drop-down of compatible components on the form, `<new …>` to create one); **RDataPreview** (public): schema (column, type, nulls, distinct), rows (virtual grid, sort / filter locally), quick stats (count, mean, median, std, min, max, top values, a histogram sparkline per column), the SQL / pipeline that produced it; opens on any source or transform selected in the designer, and on any frame while debugging (I6).

**One implementation** (rule 2): RNUM, RDATAFRAME and RPLOT's model are one implementation since 2026-10-06 (`rapidr_value::datascience`, D7); RPlot is drawn by the UI kernel's renderer on every runtime (L-FRAME, below). I7 builds on that:
- **The frame engine** (decision **D7**, decided 2026-10-06): our own engine on every runtime, `rapidr_value::datascience` — polars dropped (the L-FRAME spike measured it as a 1.5 MB brotli module that runs single-threaded — it would fit — but RapidR's tables are small and one dependency-free engine is simpler; [the record](#i7--l-frame-spike-results-2026-10-06)). I7 grows it into a columnar engine (`rapidr-frame`: typed columns, the transforms above, CSV / JSON / Parquet readers with permissive crates) behind the same members, checked by the corpus-style comparison on data programs (`examples/data/dataframe.rr` …) and the `datascience_*` conformance cases.
- **RPlot on the kernel**: a plot model in `rapidr-value` (axes, ticks, legends, marks, text) that paints the kernel's vector ops — identical on desktop and web, sharp at any scale, accessible (a summary plus the data as a table for screen readers), `SaveFig` to PNG through the CPU renderer and to SVG from the ops. Replaces plotters (desktop) and the canvas code (web).
- **The dataset model** in `rapidr-db`: `Dataset { fields, row_count, cursor, state: Browse | Edit | Insert, read(row, field), edit / post / cancel / delete / refresh, events }` implemented by DB queries / tables (editable, posting by primary key with bound parameters) and frames (read-only, or in-memory edits).

**Live at design time.** The designer evaluates the form's data components in the IDE (not in the program): sources read in the background with limits (first 10,000 rows for previews by default, a size cap, a timeout), transforms evaluated lazily, grids and plots drawn from the result, re-evaluated (debounced 200 ms) when a property or the file changes (`Watch`). **Safety**: design-time reads only files inside the project unless the user allows a path; databases open read-only at design time (SQLite `SQLITE_OPEN_READONLY`, MySQL `START TRANSACTION READ ONLY`), and a MySQL connection at design time asks once per project; design-time never runs DML.

**AI and data** (built in I8; listed here because the components must expose it): MCP tools to inspect data (schema, sample rows, statistics), run transforms or read-only queries, and create or rebuild plots and their component links, so "group sales by month and plot it as bars" becomes: inspect the source's schema → propose an `RDFCompute` (Month), an `RDFGroup` (Month, `Sales:sum`) and an `RPlot` (bar, X = Month, Y = Sales) as a diff of CREATE blocks → apply → the chart appears live in the designer. The same tools through `RAI` in users' programs. A per-project **data access level** for AI — none, schema only (the default), a sample, or full data — enforced in the tool layer, with column redaction, local models recommended for sensitive data, and the exact payload shown before sending ([docs/ide-ai.md](ide-ai.md) §6).

**Depends on.** I0 (registry), I4 (tray, pickers, designer), I6 (preview while debugging, later), the engine spike (first).

**Size.** L, 18–26 sessions, 4 lanes: engine (spike + one implementation; L-FRAME), RPlot on the kernel (L-PLOT), DB dataset + data-aware controls (L-DB), design-time evaluation + tray links + pickers + RDataPreview (L-DDESIGN).

**First public release (I7 core, ~10–14 sessions)**: the engine decision implemented (one implementation on all runtimes), RPlot on the kernel with declarative binding, `RDataFile` (CSV, JSON) and `RDBConnection` + `RDBQuery` / `RDBTable` for SQLite, `RDataFrame.Source`, `RDFFilter`, `RDFSort`, `RDFGroup`, `RDFCompute`, `RDataSource`, `RDBGrid` (read-only), live at design time, RDataPreview, the tray and the pickers. **Later (1.x)**: Parquet, MySQL at design time, `RDFJoin`, `RDFSelect` / `RDFLimit`, editable datasets with `RDBEdit` & co. and `RDBNavigator`, lookups, the AI data tools (with I8).

**Acceptance.**
- A CSV file dropped on a form becomes an `RDataFile`; pointing an `RPlot` at it (via the picker) shows the plot in the designer within 300 ms for a 100,000-row file; editing the CSV on disk updates the plot; changing `Kind` redraws at once.
- A SQLite file + `RDBQuery` + `RDataSource` + `RDBGrid` shows the rows at design time; the same form run native, interpreted and on the web shows the same rows.
- Data programs give identical output on all three runtimes (frames printed, plots compared as captures, byte-identical between desktop and web).
- Design-time evaluation never writes (a test with a DML statement in `SQL` is refused at design time, runs at run time).
- RDataPreview's stats equal the engine's `describe` for the corpus of data fixtures; it is fully keyboard- and screen-reader-operable.
- `tools/regress.sh legal` passes with the engine's (and Parquet's) dependencies.

### I7 / L-FRAME spike results (2026-10-06)

**Question** (D7): can polars — the desktop's RDataFrame engine — be the web's too, as a module loaded only by programs that use data frames, at ≤ ~3 MB brotli? And how should RPlot draw, now that the web is the UI kernel on a canvas?

**polars 0.46 on `wasm32-unknown-unknown`** (a spike crate: CSV and JSON read / write, filter, sort, group-by with sum / mean / count, a left join, mean / std / min / max / median, string upper-case, the pretty-printed table; `opt-level = "z"`, LTO, one codegen unit, `panic = "abort"`, stripped; wasm-bindgen; measured with `gzip -9` and `brotli -q 11`):

| Build | Raw | gzip | brotli |
|---|--:|--:|--:|
| polars (`lazy csv json dtype-full strings round_series abs log fmt_no_tty`) | 12.9 MB | 2.42 MB | **1.50 MB** |
| the same with `+simd128` (as the web runtime is built) | 12.7 MB | 2.39 MB | 1.49 MB |
| … `+ parquet` (zstd, lz4, snappy, brotli codecs) | 16.3 MB | 3.50 MB | **2.26 MB** |
| (after `wasm-opt -Oz`: smaller raw, *larger* compressed — not worth it) | 11.1 MB | 2.57 MB | 1.60 MB |
| for scale: today's whole web runtime `rapidrintr_bg.wasm` (compiler + VM + kernel + SQLite) | 10.2 MB | 4.34 MB | 2.71 MB |
| for scale: plotters (bitmap backend, every series, PNG encoder) | 0.79 MB | 0.22 MB | 0.16 MB |

- **Size: fits**, with room — 1.50 MB brotli with what RDataFrame uses today, 2.26 MB with Parquet (the plan's "later"); polars' monomorphized code compresses ~8.6×.
- **Build friction: low, three settings.** (1) getrandom 0.3 (through ahash) needs its `wasm_js` feature *and* `RUSTFLAGS='--cfg getrandom_backend="wasm_js"'`, getrandom 0.2 its `js` feature — the module gets its own build (its own target directory), so the flag never touches the other crates; (2) the `fmt` feature pulls crossterm (no wasm): `fmt_no_tty` prints the same tables; (3) Parquet's zstd is C: the wasm C toolchain and the `AR` shim SQLite already uses (`tools/wasm-ar.sh`). rayon has no threads on wasm and runs everything on the calling thread — no code change. A clean release build of the module takes 65–75 s on this machine (M-series), an incremental one ~20 s.
- **Speed: good.** In Node (V8), single-threaded: instantiate 15 ms; the small pipeline above 33 ms; 1 M rows written to CSV, read back, filtered, sorted and grouped in **320 ms** (100 k rows: 37 ms) — the same order as the native build (390 ms at `opt-level = "z"`).
- **Results: the same code**, so the same answers (dtype inference, float formatting, sort stability, null handling) on every runtime; the one wasm32 difference to keep in mind is a 32-bit `usize` in *our* code (polars indexes rows with `u32` on every target by default).
- **Licences: all permissive.** 180 crates in the wasm graph; `cargo deny check licenses bans` with this repository's `deny.toml` passes (MIT / Apache-2.0 / BSD-2 / BSD-3 / Zlib / BSL-1.0 / Unicode-3.0 / Unlicense; polars-arrow-format already clarified as Apache-2.0); nothing copyleft, no MPL, no cryptography compiled in (the codecs are compression only).

**Our own engine instead** (estimate for the same API): typed columns (i64 / f64 / string / bool with null masks), a CSV reader with quoting and type inference, JSON records / NDJSON through serde_json, filter / sort / group-by / join / describe / value counts / the pretty table — about 3,500–5,000 lines, 8–12 sessions plus hardening (CSV dialects, number formatting, null semantics, group and join edge cases are all ours to get right and to test), and ~0.3–0.5 MB raw / ~0.1–0.15 MB brotli on the web. Parquet would still need a reader crate (arrow-rs' `parquet` is itself large). It would win only on size, which polars already meets; it loses on cost, breadth (polars' expressions are what RDFFilter / RDFCompute compile to) and risk.

**RPlot — what each option looks like** (the same six charts in one gallery program: lines with a legend and grid, bars, a scatter with a reference line, a histogram, a pie, an area with a step line; desktop at 1× and 2× through the headless host, the web on `tests/web_kernel.html` at 1× and 2×):

| Option | Looks (1–5) | Why |
|---|:-:|---|
| plotters on the desktop (today) | 2 | A 1× bitmap: at 2× (Retina, a 200 % browser) it is stretched and blurry; text heavy and fuzzy (coverage through a square root); bars over a numeric axis (0.5 … 3.5) instead of their categories; a dense mesh for `Grid`; a saturated pure-colour palette; names it doesn't know (`royalblue`) draw black; the area and step series have no legend entries; pie labels clipped. |
| HTML canvas on the web (today) | 3 | Smooth lines and the browser's fonts, a softer palette, a better pie — but ticks at unround numbers (0.66, 86.20, 2.53), `XLim` / `YLim` ignored (the bar chart falls below its axis), and none of it the desktop's: different fonts, sizes and colours, so the "same" program shows two charts. |
| plotters on the web | 2 | Builds without friction (0.16 MB brotli) and would make both sides the same — the same blurry, plotters-styled bitmap. |
| **a plot model painting the kernel's vector ops** (the recommendation) | 5 (target) | The kernel's own renderer on both hosts (vello / vello_cpu), so the same pixels on desktop and web; drawn at the device's scale, so crisp at 1×, 1.5×, 2×, 3×; the kernel's fonts through parley; nice ticks (1-2-5 steps), category axes, light gridlines, a legend with swatches, a modern palette, the theme's colours (dark, high contrast). A QIMAGE's `LoadFromPlot` keeps its 1× pixels for `Pixel` (RapidQ's rule) and shows the chart drawn again at the screen's scale (the bitmap's high-DPI layer, as SVGs already are); `SaveFig` writes a PNG through the CPU renderer. |

**The spike's recommendation was polars everywhere** (it fits; a `rapidr-frame` crate with a pure interface — a call in with a file's bytes, a reply out — linked on the desktop and loaded on the web as `rapidrframe_bg.wasm` only for programs that use `RDATAFRAME`). It was built that way and passed the `datascience_*` conformance cases native, interpreted and on the web (worktree commits aa87635, 318a97c). **D7 was decided otherwise the same day**: our own engine (`rapidr_value::datascience`, above), for its simplicity; the numbers here stay as the record of the alternative, and the frame engine's work goes on as speed (a 1 M-row benchmark, columnar where it lags).

**The frame engine's speed, as built** (L-FRAME, after D7): the engine stores a frame by columns — each column's cells' text in one buffer with offsets and a null mask, its type and its cells as numbers worked out once and cached — and filters, sorts, groups and joins by row index (a hash join, numbers compared as numbers). Every member keeps its meaning (the `datascience_*` conformance cases unchanged, on all three runtimes). One million rows, five columns (37 MB of CSV), `cargo run --release -p rapidr-value --example frame_bench`, M-series desktop / the same engine as wasm in V8:

| Operation | Row store (before) | Columnar, desktop | Columnar, wasm (V8) | Target (desktop; web 2×) |
|---|--:|--:|--:|--:|
| load CSV text | 995 ms | 114 ms | 286 ms | 300 ms |
| filter `salary > 75000` | 169 ms | 40 ms | 84 ms | 60 ms |
| sort by salary, descending | 3,129 ms | 182 ms | 222 ms | 250 ms |
| group by dept, mean | 1,418 ms | 80 ms | 127 ms | 120 ms |
| left join on dept | 1,424 ms | 60 ms | 121 ms | 120 ms |
| describe | 592 ms | 79 ms | 127 ms | 150 ms |
| save CSV text | 713 ms | 80 ms | 177 ms | 150 ms |

(for comparison, the polars spike in V8: CSV write + read + filter + sort + group-by of 1 M rows in 320 ms.)

**RPLOT, as built**: the chart model draws the UI kernel's vector ops (`rapidr_value::datascience::plot` → `rapidr-ui-render`'s `chart.rs`), one renderer for every runtime: a QIMAGE's `LoadFromPlot` keeps the 1× pixels a program reads and is drawn again at the screen's scale (the bitmap's high-DPI layer), `SaveFig file, scale` writes a PNG. The same chart captured on the desktop's headless host and in the browser differs in **0 pixels** at 1× and at 2×. Before / after: the six-chart gallery and `examples/data/dataframe.rr` (category bars) — PNGs listed in the L-FRAME report.

### I8 — Smart (AI through MCP)

**Goals.** AI in the IDE through MCP, opt-in and transparent: an MCP server inside the IDE, started and stopped from the IDE, exposing everything as tools; a floating AI button (shown when AI is configured) opening an assistant that uses those tools; changes previewed as diffs before they apply; permission levels; multi-provider through one layer over plain HTTPS; keys in the OS keychain; the same layer as `RAI` for users' programs; AI that can drive the editor component in users' own products.

Specified in [docs/ide-ai.md](ide-ai.md). In short:
- **`rapidr-ai`**: one client, three wire formats — Anthropic Messages, OpenAI-compatible Chat Completions (OpenAI, DeepSeek, xAI Grok, Ollama's and LM Studio's `/v1`, any compatible endpoint), Google Gemini `generateContent` — streaming (our own SSE parser), tool calls normalized, images in and out, usage and cost reporting; `ureq` + `rustls` on the desktop (both already in the workspace), `fetch` on the web. No vendor SDKs.
- **`rapidr-secrets`**: macOS Keychain, Windows Credential Manager, Linux Secret Service (`keyring-core` 1.x with its `apple-native`, `windows-native` and `zbus-secret-service` stores, all MIT / Apache); web: in memory for the session, opt-in persistence encrypted with a passphrase (WebCrypto), or a local `rapidr ai-proxy` holding the key in the desktop keychain. Keys never in projects, bundles, logs or MCP results.
- **`rapidr-mcp`**: our own JSON-RPC + MCP implementation (the official Rust SDK, `rmcp`, is Apache-2.0 — allowed — but built on tokio and not usable in the browser; the protocol subset we need is small); transports: **stdio through `rapidr mcp`** (a proxy that connects to the running IDE over a per-user local socket — a Unix socket with 0600 permissions / a named pipe with a current-user ACL — authenticated with a token), and optionally **Streamable HTTP on 127.0.0.1** (random port, bearer token, Origin and Host checks, off by default). The IDE's MCP panel starts / stops it, shows the connection command for Claude Code / Claude Desktop, the connected clients, and the audit log.
- **Tools** (each a component's tool provider, so the same tools exist for users' programs — §I9 and [docs/ide-ai.md](ide-ai.md) §4): project and files (list, read, search, outline), edits (apply as diffs), forms and components (create, set and link properties, add handlers), run / stop / build, debug (breakpoints, step, stack, variables, evaluate), diagnostics and output, the running program (its accessibility tree, screenshot, click / type through the session), data (schema, sample, stats, transforms, queries, plots).
- **The assistant**: a floating button → a docked chat panel (`RAIChat`), context chips (current file, selection, form, diagnostics), every edit shown in RDiffView before it applies (accept all / per hunk / reject), checkpoints and one-step undo of an AI turn.
- **Permissions**: per client and for the built-in assistant — read-only; ask before edits (default); edit with preview; ask before running (default); run sandboxed (no network, files only in the project) — and the "native-privileged" project flag (SEC-10: `RUSTSTART`, `DECLARE … LIB`) always asks.
- **Transparency**: opt-in (nothing is sent until a provider is configured and the user enables AI for the project), "what was sent" for every request (the exact payload, minus the key), a per-project context include / exclude list, the data access level (§I7).
- **RAI** (ROADMAP Phase 5) is the same provider layer and tool machinery as a component; its tool-calling, `'@tool` SUBs and "AI that uses the running app" build on I8's pieces.
- **AI driving the editor in users' products**: RCodeEditor, RFormDesigner, RDataPreview and the other components expose their tools to `RAI` when the developer allows it (`AI.AttachComponent CodeEditor1, "edit"`), with the same preview and veto events (`OnToolCall`).

**Depends on.** I0 (registry for the system prompt), I1–I4 and I6 for the tools they expose (tools land as their components do), I7 for the data tools.

**Size.** L, 16–24 sessions, 4 lanes: provider layer + secrets (L-AI; independent, can start early), MCP core + transports + permissions + audit (L-MCP), tool providers across components (L-TOOLS, coordinated with each component's owner), assistant UI + diff flow (L-AIUI).

**Acceptance.**
- Provider conformance against recorded fixtures (no network in `tools/regress.sh`) for each wire format: streaming text, tool calls, errors, rate limits, cancellation; a manual live check per provider before release.
- MCP: Claude Code and Claude Desktop connect through `rapidr mcp`, list tools, edit a form, run the program and read its output; a connection without the token, from another user, or from a browser origin is refused (tests); the official MCP Inspector passes against our server (dev-time).
- Every mutating tool goes through the permission tier and the diff preview; an AI turn is undone in one step; the audit log lists every call.
- Keys: never found in project files, bundles, logs, crash reports or MCP results (a test greps all outputs of a session for a canary key).
- Data access levels enforced for built-in and external clients alike (a test per level).
- The threat-model tests in [docs/ide-ai.md](ide-ai.md) §7 (prompt injection in a project file, a malicious MCP client, exfiltration through a run) pass.

### I9 — Extensions

**Goals.** Users add languages, themes, snippets, component templates, and providers (completion, hover, diagnostics, formatting, commands, panels, AI tools), ideally written in RapidR itself; packaged, permissioned and sandboxed.

Specified in [docs/ide-components.md](ide-components.md) §5. In short: **declarative contributions** (language definitions, colour schemes, snippets, toolbox templates, keybindings) need no code and no permissions; **code contributions** are RapidR bytecode run in their own VM in a **capability-filtered host** (ROADMAP Phase 5's `SandboxHost`: no files, network, processes, FFI, `RUSTSTART` or `DECLARE … LIB` unless the manifest asks and the user grants; fuel and memory limits; on the web, a worker) and talk to the IDE only through the `IDE` object (events such as `OnCompletionRequest`, methods such as `AddCompletion`, `RegisterCommand`, `AddPanel`). Packages are `.rrext` zips (manifest `extension.toml`, bytecode, assets, licence), installed per user from a file or a URL, with a permission prompt, version pinning and an optional signature (an unsigned package shows a warning).

**Depends on.** I1–I3 (the extension points), I8's tool providers (for AI-tool contributions).

**Size.** M, 8–12 sessions, 2 lanes: sandboxed host + IDE API (L-EXTHOST), packaging + install UI + declarative contributions (L-EXTPKG).

**Acceptance.** A sample extension written in RapidR adds a language (declarative) and a diagnostics provider (code) on both hosts; it can't open a file or a socket without the grant (tests for each capability); a runaway extension is stopped by fuel without freezing the IDE; uninstall leaves nothing behind.

---

## 5. Data science in the IDE, at a glance

The data-science stack (RNum, RDataFrame, RPlot today) is first-class across stages rather than a stage of its own:

| Capability | Stage | First release? |
|---|---|---|
| One frame engine and one RPlot implementation on all runtimes | I7 | Yes |
| Sources: CSV, JSON, SQLite files and connections → `RDataFrame` | I7 | Yes |
| Sources: Parquet, MySQL at design time | I7 | Later |
| Transforms: filter, sort, group / aggregate, computed columns | I7 | Yes |
| Transforms: join, select / rename, limit | I7 | Later |
| Outputs live at design time: `RPlot` bound to a frame, `RDBGrid` | I7 | Yes |
| Data preview panel (schema, rows, quick stats) | I7 | Yes |
| Frames in the debugger (data tips, variables → preview) | I6 + I7 | Later (1.1) |
| Completion of column names in conditions / expressions | I3 + I7 | Later |
| AI: inspect data, run transforms / queries, build plots and links | I8 | Later (with I8) |
| AI data access levels and "what will be sent" | I8 | Later (with I8) |
| The same through `RAI` in users' programs | I8 / Phase 5 | Later |

---

## 6. Cross-cutting

### 6.1 Accessibility

The IDE is fully usable with a keyboard and with VoiceOver, NVDA, JAWS, Narrator and Orca (desktop) and screen readers over the web host's mirror:
- Every panel is a kernel component with an accessibility description; the dock manager exposes areas as landmarks (groups with names), F6 / Shift+F6 cycle areas, Ctrl+Tab cycles documents, every command has a shortcut or is in the command palette.
- Editor: text runs, line / column, diagnostics, completion (I2). Designer: components as a tree (name, type, position, size), keyboard placement and resizing with announcements (I4). Inspector: a grid with row names, editors as native-like controls. Debugger: the current line announced on stop. Assistant: streaming replies in a polite live region, diffs readable as text.
- High contrast theme, focus rings everywhere, no colour-only information (diagnostic icons and shapes, not just colour), respects reduced motion, UI scale (Ctrl+= / Ctrl+−) and the OS text size.
- Verified per stage: `tests/web_a11y.mjs`-style comparisons of Chrome's tree with the kernel's, AccessKit unit tests, the macOS AX probe and the Windows UIA probe on the IDE's windows, and a hands-on screen-reader pass per stage on each OS before its box is ticked.

### 6.2 Performance targets

Reference machines: an Apple M1 MacBook Air (8 GB), a 2020 Intel Core i5 laptop on Windows 11, an ARM Ubuntu VM; Chrome / Firefox / Safari current. "Desktop" = the winit host, "web" = the canvas host with wasm SIMD.

| Measure | Desktop | Web |
|---|---|---|
| Cold start to an interactive empty workspace | ≤ 1.0 s (warm ≤ 0.4 s) | ≤ 2.5 s first visit on 50 Mbit/s, ≤ 1.2 s cached |
| Open a 50-file project, first editor painted | ≤ 0.5 s | ≤ 1.0 s |
| Typing latency, key → pixels on screen (100,000-line file) | p50 ≤ 8 ms, p99 ≤ 16 ms | p50 ≤ 16 ms, p99 ≤ 33 ms |
| Open a 10 MB / 200,000-line file to first paint | ≤ 300 ms | ≤ 800 ms |
| Scroll a 1,000,000-line file | 60 fps, no frame > 16 ms | 60 fps, no frame > 33 ms |
| Editor memory | ≤ 3 × file size + 50 MB | same |
| Completion popup after trigger (20,000-line project) | p95 ≤ 50 ms | p95 ≤ 100 ms |
| Diagnostics after typing stops | ≤ 300 ms | ≤ 500 ms |
| Designer drag, 300 components | 60 fps, guides ≤ 2 ms per move | 60 fps |
| Run (F5) to the program's first form, small program | ≤ 300 ms | ≤ 500 ms |
| Design-time plot of a 100,000-row CSV | ≤ 300 ms | ≤ 1 s |
| IDE idle memory | ≤ 200 MB | ≤ 300 MB tab |
| IDE CPU when idle (caret blinking) | < 1 % | < 2 % |

Measured by a `tools/regress.sh perf` stage (timings from the hosts' frame clocks and `performance.now()`, a scripted typing test through the test hooks, the headless host for desktop frame times) with the numbers recorded in each stage's results; a regression > 20 % fails the stage.

### 6.3 Security

- **The program under development** runs isolated (§3.3): its own process or an opaque-origin sandboxed frame; it never sees the IDE's settings, keys or MCP token.
- **Projects are untrusted input**: opening a project never runs its code; `$INCLUDE` is confined to the project and the RapidR libraries in IDE / MCP contexts (SEC-10); projects using `RUSTSTART` or `DECLARE … LIB` are flagged "native-privileged" and building or running them asks once per project; design-time data access is read-only and inside the project (§I7).
- **Extensions** run sandboxed with declared, granted capabilities (§I9).
- **MCP and AI**: the local socket / loopback-only server, the token, Origin / Host checks (the build server's guard, `crates/rapidr-buildserver`, reused), permission tiers, diff previews, the audit log, sandboxed AI-initiated runs, keys in the OS keychain; the threat model in [docs/ide-ai.md](ide-ai.md) §7, folded into ROADMAP Phase 6's `docs/THREAT_MODEL.md`.
- **The web IDE**: a strict CSP (no `unsafe-eval` in the IDE page; the program's frame has its own), no third-party scripts or fonts, Trusted Types (Phase 6).
- **Fuzzing** (Phase 6's targets plus): the language service on random edits, the session protocol's and MCP's message parsers, the project loader, the language-definition loader.

### 6.4 Testing, per stage, on desktop and web

| Layer | How |
|---|---|
| Models (rope, undo, tokenizer, guides, sync, dock layout, registry, frame engine) | Rust unit and property tests (`proptest` is MIT / Apache, dev-only), `cargo test --workspace`, and the wasm32 check |
| Components | Fixtures in `tests/fixtures/` + cases in `tests/gui_parity_cases.mjs` (events, dumps, captures, a11y): native and interpreted on the headless desktop host at 1× and 2×, the web host byte-identical (`web_gui_parity.mjs`), Chrome's accessibility tree equal to the kernel's (`web_a11y.mjs`) |
| The IDE as a program | The same machinery on the IDE itself: `RAPIDR_TEST_EVENTS` scripts drive the shell (open a project, drag a component, type, run), dumps and captures checked on both hosts — the IDE suites replace `tests/web_ide_*.mjs` one for one |
| Real input | `tools/real_input.py` (macOS Quartz events) on the desktop IDE; Playwright's real mouse / keyboard / CDP IME on the web; by hand on Windows and Ubuntu VMs per stage |
| Accessibility on real platforms | `tools/macos/ax_dump.swift`, `tools/windows/uia_probe.ps1`, plus a screen-reader session per stage (VoiceOver, NVDA; JAWS / Orca / TalkBack before the release) |
| Language service | Golden marker files (`tests/langsvc/`), the corpus without panics, LSP request / response fixtures |
| Designer sync | The corpus round trip (byte-identical saves), scripted edits compared with the corpus comparison tool |
| Debugger / session | Scripted sessions on both transports; DAP fixtures |
| AI / MCP | A mock provider server replaying recorded responses (no network in the gate); an MCP client test harness (and the MCP Inspector by hand); permission and threat-model tests |
| Performance | `tools/regress.sh perf` (§6.2) |
| Legal | `tools/regress.sh legal` with every new crate; `THIRD_PARTY_NOTICES.md` regenerated |

New `tools/regress.sh` stages: `ide` (the IDE's scripted suites on both hosts), `langsvc`, `perf`. A stage's ROADMAP box is ticked only with its suites in the gate and green.

### 6.5 RapidQ and RapidR components together

Programs mix Q and R components freely (one component, two names: `canonical_type_name`). The IDE's rules, specified in [docs/q-and-r-components.md](q-and-r-components.md): the toolbox groups components as "RapidQ" and "RapidR"; the designer writes RapidQ components under their Q names (so a RapidQ-only program stays a plain RapidQ program) and R-only ones under R names, and never renames what the source already says; the inspector and IntelliSense treat both names as the one component, marking RapidR-only members of RapidQ components ("RapidR extensions"); an optional "RapidQ-compatible project" setting reports every RapidR-only component, member, builtin, statement and directive (and Q names RapidQ doesn't have, such as `QPLOT`) with code actions, without changing how the program compiles or runs. Stages: the origin data in I0, the toolbox and inspector in I1, completion and the compat diagnostics in I3, the writing rules in I4.

### 6.6 Candidate dependencies and their licences

Checked with `cargo info` on 2026-10-05; versions move, so `tools/regress.sh legal` is the authority.

| Need | Candidate | Licence | Verdict |
|---|---|---|---|
| Rope | `ropey` 1.6.1 (2.0.0-beta.1) | MIT (2.0: MIT OR Apache-2.0) | **Use** (behind a `Buffer` trait) |
| Rope, byte-indexed | `crop` 0.4.3 | MIT | Alternative |
| Incremental parsing | `tree-sitter` 0.27 | MIT (grammars vary) | **Don't**: C runtime vs `wasm32-unknown-unknown`, no RapidQ grammar, duplicates our parser |
| Regex | `regex` | MIT OR Apache-2.0 | Use (search, language definitions) |
| LSP | `lsp-server` 0.10 / `lsp-types` 0.97 | MIT OR Apache-2.0 / MIT | Use (desktop only) |
| DAP | `dap` 0.4.1-alpha1 | MIT OR Apache-2.0 | **Don't** (alpha); our own types from DAP's JSON schema |
| MCP | `rmcp` 3.5.1 (official SDK) | Apache-2.0 | Licence fine; **don't** (tokio-based, not for the browser, large); our own on `rapidr-jsonrpc`; `rmcp` as a dev-only conformance client at most |
| JSON-RPC | `jsonrpc-core` 18 | MIT | Unmaintained; our own (small) |
| Keychain | `keyring` 4.2 → `keyring-core` 1.0 + `apple-native-keyring-store` 1.1, `windows-native-keyring-store` 1.0, `zbus-secret-service-keyring-store` 1.0 (via `zbus` 5, MIT) | MIT OR Apache-2.0 | **Use** (not the `dbus-secret-service` store: it links the C libdbus, AFL / GPL) |
| HTTPS | `ureq` 2.12 + `rustls` 0.23 + `ring` (Apache-2.0 AND ISC) | MIT OR Apache-2.0 | Already in the workspace; prefer `rustls-platform-verifier` (MIT OR Apache-2.0) over `webpki-roots` (CDLA-Permissive-2.0, outside the stricter list) for new code |
| JSON | `serde`, `serde_json` | MIT OR Apache-2.0 | Already in the workspace |
| TOML (project file) | `toml` | MIT OR Apache-2.0 | Use if D2 picks TOML |
| Data frames | our own (`rapidr_value::datascience`, D7); Parquet readers later | — | Parquet: check the codecs' crates (zstd, lz4, snappy, brotli — each permissive, verify). The alternative measured: `polars` 0.46 (MIT) — 180 crates on wasm, `cargo deny` clean |
| Property tests | `proptest` | MIT OR Apache-2.0 | Dev-only |
| Icons | Lucide (ISC), Tabler Icons (MIT) | ISC / MIT | Use one (D8); **not** VS Code's Codicons (CC-BY-4.0, outside the list) |
| Fonts | JetBrains Mono, Cascadia Code, Inter, the shipped Liberation (all OFL-1.1) | OFL | D8 |

---

## 7. Lanes and order

The lanes are named in each stage. Waves let several agents work in parallel without touching the same files; each lane owns its crates / modules, and **shared files have one integrator** (the pattern of the host plans):

- Shared files (integrator only; lanes hand over small patches): `crates/rapidr-value/src/objects/mod.rs` (the `Object` enum and creation table), `crates/rapidr-ui-kernel/src/components/mod.rs` (`KINDS`), `crates/rapidr-ast/src/lib.rs` (`COMPONENT_TYPES` until it is generated), `crates/rapidr-runtime-core/src/object.rs` and `crates/rapidr-runtime-web/src/object_web.rs` (dispatch), `crates/rapidr-value/src/objects/a11y.rs`, `tools/regress.sh`, `Cargo.toml` (workspace members), `ROADMAP.md`, `CHANGELOG.md`.
- The parser (`rapidr-lexer`, `rapidr-parser`, `rapidr-preprocessor`) and the VM (`interpreter/rapidr-vm`, `rapidr-bcgen`) each have one owner at a time (L-PARSE, L-VM), coordinated with any compatibility work.

| Wave | Lanes in parallel (owner of) | Needs |
|---|---|---|
| **A** (now) | L-REG (`rapidr-lang`, generators) · L-PARSE (lexer trivia, origin map, lexer recovery, semantic model export) · L-SESS (`rapidr-project`, `rapidr-session`, the VM hosts' session side) · L-EDCORE (`rapidr-editor`) · L-DOCK (dock model in `rapidr-value`, `dockmanager` kernel component) · L-FRAME (the engine spike, D7) | — |
| **B** | L-VM (breakpoint files, pause, evaluator, set variable) · L-PANELS (inspector, toolbox, project tree, toolbar, console components) · L-EDVIEW (virtualized editor view, popups, diff view) · L-LS (semantic model, diagnostics) · L-DMODEL (designer model, guides, commands) · L-PLOT (RPlot on the kernel) · L-AI (`rapidr-ai`, `rapidr-secrets`; independent, low priority until wave E) | A's registry / parser / session / editor core / dock as noted per stage |
| **C** | L-SHELL (`ide/` shell, commands, settings, themes, icons) · L-WEB (web page, storage, run on the web) · L-SYNC (two-way CREATE sync) · L-DVIEW (designer view, tray, menu / tab-order editors) · L-LSFEAT (completion, hover, navigation, rename, formatting) · L-DBGUI (debugger views, immediate window) · L-DB + L-DDESIGN (dataset model, data-aware grid, design-time evaluation, RDataPreview, pickers) | B |
| **D** (release hardening) | Parity checklist and deletion of `web-ide/` + `examples/ide.rr` (single owner) · L-LSP (`rapidr lsp`, VS Code client; `rapidr dap` if time allows) · L-EDA11Y (accessibility pass across the IDE, IME by hand) · performance pass (`tools/regress.sh perf`) · release packaging (`tools/release/`) | C |
| **E** (after the first release) | L-REMOTE + L-VM (I5) · I6's advanced items + DAP · I7's later items · L-MCP, L-TOOLS, L-AIUI (I8) · L-EXTHOST, L-EXTPKG (I9) | D |

Order rationale: the registry, the parser-for-tools and the session protocol are on every critical path, so they start first; the editor core and the dock manager are pure models with no dependencies; the frame-engine spike decides I7's shape early because it touches the build and the wasm size; AI is independent but deliberately not on the first release's path.

---

## 8. The first public release

**Bar: I0, I1, I2, I3, I4, basic I6, and I7's core** — plus the HTML IDE deleted, the accessibility and performance targets met, and RapidQ compatibility complete on the three runtimes (ROADMAP's release item).

- I3 is in fully (rename and references are cheap once the semantic model exists, and they are what "pro" means in an IDE); the formatter may slip to 1.1.
- I6 basic (§I6): breakpoints with conditions, stepping, pause, call stack, locals / globals / watches evaluated by the VM, data tips, break on error, the immediate window (it shares the watches' evaluator). DAP and logpoints / hit counts may slip to 1.1.
- **I7's core is argued in, not out**: linked, live data at design time is the feature the user named as the Delphi / Lazarus experience, the data-science stack is first-class, and RPlot / RDataFrame must become one implementation anyway (rule 2) before a release can claim identical runtimes. It reuses I4's tray and pickers, so its marginal cost is moderate (~10–14 sessions).
- **I8 is out** of the first release: AI is ROADMAP's Q2 2027 line, it needs the tools of every other stage, and its security review deserves its own release. **Stretch**: a read-only MCP server (project, files, diagnostics, outline, data schema) behind a setting, if wave D has room — it lets Claude Code and others read a RapidR project well from day one with little risk.
- **I5 and I9 are out** (1.1 and later).
- **The VS Code extension ships too (the user, 2026-10-06)**: for people who prefer VS Code, the first release includes the extension as a compiled `.vsix` on the GitHub release (and on the VS Code Marketplace and Open VSX once the user has publisher accounts), as an LSP client of `rapidr lsp` — the same IntelliSense as RapidR Studio (completion of builtins, keywords, components and their members, the user's own variables, SUBs, FUNCTIONs, TYPEs; hover; signature help; go to definition; references; rename; diagnostics; outline; formatting) — plus run / build commands and debugging through `rapidr dap`.

Release notes, per the project's messaging: full RapidQ compatibility on all three runtimes (native compiler, interpreter, web), extended (data science, data components, AI to come), an original implementation in pure Rust — not a clone of RapidQ.

---

## 9. Decisions

**Decided by the user (2026-10-05):** D1 — the IDE is **RapidR Studio**; D9 — no default AI provider, a first-run chooser listing local models first; D12 — predefine `RAPIDR` (`$IFDEF RAPIDR`); D14 — the first public release is §8's scope (I0–I4, basic I6, I7's core). The other rows go with their recommendation unless the user says otherwise.


| # | Decision | Options | Recommendation |
|---|---|---|---|
| **D1** | The IDE's name | "RapidR IDE"; "RapidR Studio"; another | "RapidR Studio" (reads as a product; `rapidr ide` stays the command) |
| **D2** | Project file format | TOML `.rrproj` v2; JSON `.rrproj` v2 (as v1); no project file (folders only) | TOML v2 (diff-friendly, comments), reading JSON v1; plain `.bas` / `.rr` files still open without a project. Also: confirm whether RapidQ's own IDE project files (if any) should be read — ROADMAP still asks for samples |
| **D3** | How much of the IDE is written in RapidR | (a) all Rust, components exposed to RapidR; (b) components in Rust, the shell in RapidR; (c) most of the IDE in RapidR | (b): maximum dogfooding at the shell level, performance and accessibility in Rust components. Risk: RapidR has no modules / namespaces for a large program — mitigated by `$INCLUDE` files and naming conventions; if the shell's glue proves slow, it moves into components |
| **D4** | Default document mode | MDI windows (cascade / tile); tabbed documents | MDI by default (the user's direction), tabs one click away |
| **D5** | Registry source format | Rust tables; TOML / RON data files compiled by `build.rs` | TOML data files (reviewable, generators in Rust, tests tie them to both runtimes) |
| **D6** | Rope | `ropey` 1.6; `ropey` 2 (beta); `crop` | `ropey` 1.6 behind a trait, revisit at ropey 2.0 |
| **D7** | The one data-frame engine | polars everywhere (wasm spike); our own engine everywhere | **Decided (2026-10-06): our own engine everywhere** — `rapidr_value::datascience` (RNUM, RDATAFRAME, RPLOT's model), polars and ndarray dropped: RapidR programs' tables are small, one dependency-free engine is simpler, and two implementations had drifted apart; a columnar engine can replace the frame behind the same members later. **The alternative, measured** (L-FRAME spike, [record](#i7--l-frame-spike-results-2026-10-06)): polars 0.46 builds for `wasm32-unknown-unknown` with three settings and fits as a lazily loaded module — 12.9 MB raw, 2.43 MB gzip, **1.52 MB brotli** (2.26 MB with Parquet), single-threaded (no threads needed), 1 M rows written, read, filtered, sorted and grouped in 320 ms in V8, every licence permissive; it was built and passed the conformance cases on all three runtimes before this decision. RPLOT: drawn by the UI kernel's renderer on every runtime (pixel-identical desktop / web) |
| **D8** | Icons and fonts | Lucide (ISC) or Tabler (MIT); editor font JetBrains Mono / Cascadia Code / Liberation Mono; UI font Inter / the shipped Liberation Sans | **Decided by the user (2026-10-06): RapidR's own icon set** — every action, component, file type and glyph drawn as our own SVGs on one grid matching the brand (design/brand), MIT, used by the IDE, the manual, the website and the VS Code extension; no Lucide / Tabler. Fonts as recommended: JetBrains Mono for the editor, Inter for the chrome |
| **D9** | Default AI provider | none until configured; a local model (Ollama / LM Studio) first; Anthropic first | No default: a first-run chooser listing local models first (privacy) and the cloud providers equally; per-project opt-in |
| **D10** | Extension sandboxing | RapidR bytecode in a capability-filtered VM only; also WebAssembly extensions; also native | RapidR bytecode only (sandboxable on both hosts, written in RapidR as wished); WebAssembly considered later; never native |
| **D11** | Extension distribution | files / URLs only; a curated index (a Git repository of manifests); a store | Files / URLs plus a curated index later; signatures optional with a warning when unsigned |
| **D12** | `$IFDEF RAPIDR` | Predefine `RAPIDR` in the preprocessor (additive) so RapidQ-compatible projects can guard extensions; don't | Predefine it (RC.EXE doesn't define it, so guarded code is skipped there); needs the user's OK as a language change |
| **D13** | `QPLOT`-style names (a Q prefix on an R-only component) | keep accepting silently; accept with a warning in RapidQ-compatible projects; reject | Accept (no behaviour change), warn in RapidQ-compatible projects, the designer never writes them |
| **D14** | First release scope | as §8; without I7; with the read-only MCP server | As §8 |

---

## 10. Risks

1. **Two-way sync on real-world code** (the most serious): RapidQ programs build forms in many styles. Mitigation: the corpus round trip as the acceptance test from day one, read-only mode for what the designer can't represent, minimal text patches only.
2. **The editor's accessibility and IME across platforms**: text runs in AccessKit and the web mirror's window of lines are new ground. Mitigation: hands-on screen-reader and IME passes per stage, not at the end; VS Code's proven approach on the web.
3. **Parser changes touching compatibility**: trivia and the origin map touch the lexer and preprocessor every program uses. Mitigation: one owner, no-behaviour-change commits checked by conformance, the corpus comparison and the web parity suite.
4. **Size of the web IDE**: the kernel runtime is already ~2 MB brotli; the IDE adds the language service, the editor and fonts. Mitigation: lazy modules (the data engine, AI, fonts on demand), the web host plan's size levers, a size budget per release.
5. **The frame engine on wasm** (D7): resolved — our own engine (`rapidr_value::datascience`) runs on every runtime; the risk left is growing it into a columnar engine without changing what programs see (the `datascience_*` conformance cases hold it).
6. **A RapidR-written shell at scale** (D3): maintainability of a large BASIC program. Mitigation: the language service itself, conventions, moving heavy glue into components.
7. **AI safety and cost**: prompt injection, runaway tool loops, surprise bills. Mitigation: [docs/ide-ai.md](ide-ai.md)'s tiers, previews, caps, usage display.
8. **Scope**: ten stages. Mitigation: the release bar in §8, waves with clear owners, gaps closed before new work.

---

## Results

(Appended per stage as work lands, with dates, sizes, test counts and what changed in the plan.)

### I2 / L-EDCORE results — the editor model (2026-10-06)

**What landed.** `crates/rapidr-editor` (new workspace member; ~4,400 lines of Rust, 11 language definitions in ~1,000 lines of TOML, ~1,000 lines of tests, a benchmark). GUI-free, no runtime or host dependency; builds for `wasm32-unknown-unknown` (`cargo check -p rapidr-editor --target wasm32-unknown-unknown`). Nothing uses it yet: RCODEEDITOR (`rapidr_value::objects::{code,textedit}`, kernel `codeedit.rs`) is unchanged, so every `code_editor` fixture is untouched; L-EDVIEW wires it in.

| Module | What it is |
|---|---|
| `buffer` | The `Buffer` trait and `RopeBuffer` (ropey 1.6.1 with `cr_lines` + `simd`, without `unicode_lines`: lines break at LF, CR LF and a lone CR only). Byte offsets; `utf16_position` / `offset_from_utf16` for LSP and the web; `clamp` never leaves a caret between CR and LF; `LineEnding::detect` (the most common break) kept per document, typed by Enter, never normalized unless `convert_line_endings` is asked (one undo step). |
| `selection`, `transaction` | `Selection { anchor, head, goal }`, `Selections` (sorted, merged when overlapping or when a caret touches a selection; two touching selections stay apart; primary tracked). `Change { range, insert }`, `ChangeSet` (sorted, overlaps refused with the two indexes, `invert`, `map_pos` with before / after association), `Transaction { changes, selections_before, selections_after }`. |
| `history` | The undo tree: undo / redo walk one branch (redo follows the last made or visited child), an edit after undo starts a branch and keeps the old one, `goto_revision` reaches any node (undo to the common ancestor, redo down). A revision is a list of steps, so grouping needs no change-set composition. Typing joins the current revision while it is the same kind, the caret is where the last step left it, under 400 ms since, and no new word starts (a word character after a non-word one, or a line break); Backspaces and Deletes group the same way; commands, paste, `ApplyEdits`, replace-all are one step each. Clocks are the caller's (`now_ms`): `Instant::now` panics in the browser. |
| `document` | All of it together, and every editing command built per selection by one routine (`edit_each`), so multi-cursor is the single-caret code path: typing (auto-closing pairs: insert, overtype, surround a selection; not inside strings / comments, a quote not after a word; outdent while typing `END SUB` / `NEXT` / `}`), Enter (keeps the indentation, +1 level after a line matching `indent.increase`, a bracket pair split onto its own lines), Backspace (CR LF at once, an empty pair at once, back to the previous tab stop in the indentation), Delete, delete word, delete lines, Tab / indent / outdent, toggle line comment, paste (one line per caret when the counts match), copy text, `ApplyEdits`, `replace_range`, `set_text`; moves (character, word, line with a goal column, smart Home, End, document), add cursor above / below, Ctrl+D (whole words when started from a word), select all occurrences; `is_modified` / `mark_saved`. |
| `search` | `SearchQuery` (literal / regex, case, whole word; `with_options` reads RCodeEditor's `"case, word, regex"`), `Searcher` (the `regex` crate, multi-line with CR LF-aware `^` `$`), find all / next / previous with wrap, in the selections, incremental from an origin, replace next / all (one undo step) with `$1` / `${name}` in regex mode, literal otherwise. Find in Files stays I1's. |
| `lang` | The declarative definitions of ide-components.md §2, as TOML: `[language]`, `[brackets]`, `[indent]`, `[folding]`, `[keywords]`, `[defaults]` (a state's text token), `[[states.*]]` rules (`match`, `token`, `keywords`, `groups` per capture group, `push` / `pop` / `next`, `embed` + `end` + `end_token`, `include`), `[[snippets]]`. Errors name the rule (`rust: states.root rule 3: …`); a rule that can match nothing must change state; `deny_unknown_fields` everywhere. Token kinds: the fixed vocabulary plus custom names under it (`keyword.tag`, `string.url`), each with its fallback chain for colour schemes. |
| `lang::tokenizer` | One line at a time from a state stack (≤ 64 deep), each state's rules compiled into one multi-pattern regex (regex-automata's meta engine: leftmost match, ties to the earlier rule — one search per token says which rule matched); embedded regions as stack markers whose end pattern is searched first; at most 20,000 bytes of a line are tokenized; empty matches bounded. |
| `highlight` | Incremental colouring storing only each line's end state (an interned stack, `u32`), lazily (`ensure`, idle `advance`); after an edit lines are re-tokenized until — past every edited line — a line ends as it did before. `line_spans(line, state) → (tokens, state)` is `rapidr_value::objects::code::spans`'s contract. `take_restyled` tells the view which unedited lines changed colour. |
| `structure` | Fold ranges by markers (`SUB … END SUB`, ignoring markers in strings / comments), by multi-line bracket pairs, by indentation, and for embedded regions (`RUSTSTART … RUSTEND`, `<script>`); bracket matching that skips (or stays inside) strings and comments, scanning at most 20,000 lines. |
| `snippet` | Snippet bodies expanded at a caret (indentation, indent unit, line ending; `$1`, `${1:default}`, `$0`), `insert_snippet` at every selection selecting the first stop; tab-stop navigation is the view's. |

**Built-in languages** (`languages/*.toml`, `Languages::builtin()`, by id / extension / file name): RapidQ / RapidR BASIC (`rapidq-basic`: comments `'` and `REM`, directives, `$ESCAPECHARS ON … OFF` switching strings to backslash escapes, SUB / FUNCTION names, labels, `&H` numbers, Rust between `RUSTSTART` and `RUSTEND`), plain text, JSON, SQL, CSV, Markdown, HTML (CSS in `<style>`, JavaScript in `<script>`), CSS, JavaScript (template literals with `${ }` across lines), TOML (also `.rrproj`), Rust (nested block comments, raw strings). BASIC's keyword groups are **the compiler's own lists until L-REG lands** (`src/lang/basic.rs`: the lexer's keywords, `rapidr_ast::COMPONENT_TYPES` and their Q names, `rapidr_bytecode::builtins::BUILTINS`), with tests that fail when they drift; the registry replaces that file.

**Tests** (`cargo test -p rapidr-editor`, in `tools/regress.sh unit` through `--workspace`): 14 unit, 8 languages, 11 document, 6 search / structure, and 4 property tests (proptest, dev-only; 256 cases each by default, ~3 s in debug; run at 20,000 cases in release while developing): random edits, moves, undo / redo and jumps through the tree restore each revision's exact text and the selections before it; typing / Backspace / Delete at several carets, and typing over several selections, equal the same key at each alone; after random edits with lazy colouring in between, every line's tokens and start state equal a fresh full tokenize. The last one found a real bug (re-tokenizing that stopped short lazily could later converge above a line whose stored state was computed from a since-changed start); fixed, its seed kept in `tests/props.proptest-regressions`.

**Benchmark** (`cargo run --release -p rapidr-editor --example editor_bench`, exits 1 when a target is missed; `-- --quick` for 20,000 lines). 200,009 lines / 10.7 MB of generated BASIC with CR LF, Apple M4 Max, on a heavily loaded machine (load average ~200 from parallel lanes, so the slower items vary up to 2×):

| Measure | Result | Target (the model's share of §6.2) |
|---|---|---|
| Open + first screen coloured (languages loaded on first use) | 24–31 ms | ≤ 300 ms ✓ |
| Typing in the middle, key → the line's tokens: p50 / p99 (1,800 keys incl. Enter with auto-indent) | 0.003 / 0.012 ms | p99 ≤ 2 ms of the 16 ms ✓ |
| Memory: the document (rope + states + languages) / peak with the file text, undo and searches | 16.0 / 41.3 MB | ≤ 3 × 10.7 + 50 MB ✓ |
| Colour every line (idle, after the first paint) | 150–375 ms | — |
| Undo / redo 302 steps | 1.2 / 1.1 ms | — |
| Find all "PRINT" (55,992) / a regex (26,684) / next whole word | 4.5 / 3.6 / 0.05 ms | — |
| Replace all with groups (26,684, one step) / its undo | 25–128 / 15 ms | — |
| Fold ranges (8,181), all lines coloured | 49–90 ms | — |
| Type at 10,000 carets / move them a word | 16–24 / 21–32 ms | — |

The view's layout and painting come on top (L-EDVIEW's numbers, both hosts); the web numbers of the model need the wasm build in a page (L-EDA11Y's perf harness). The `tools/regress.sh perf` stage can run this example as is.

**Dependencies** (all inside the stricter list of §1.5; `cargo deny check` ok; `THIRD_PARTY_NOTICES.md` regenerated): shipped — ropey 1.6.1 (MIT), str_indices 0.4.4 (MIT OR Apache-2.0), toml 1.1.6 + toml_parser / toml_datetime / serde_spanned (MIT OR Apache-2.0), winnow 1.0 (MIT); regex, regex-automata, serde already in the tree. Dev-only: proptest 1.11 (+ rand 0.9, rand_xorshift, unarray; MIT OR Apache-2.0). Adding toml 1.1.6 moved the lock's existing toml_parser 1.1.0 → 1.1.3 and toml_datetime 1.1.0 → 1.1.1. `tools/regress.sh legal`: deny and notices ok; `check_notices.py`'s `bundle-bc` case needs `tools/build_web_artifacts.sh` first (not built in this worktree), every other case ok.

**Left for the next lanes.** L-EDVIEW: the kernel view over `Document` (RCODEEDITOR's `TextEdit::code()` buffer replaced, its API answered from the model: `SelStart` / `SelLength` are characters — `Buffer::byte_to_char`), colour schemes per theme (token kinds' fallback chains are ready), the snippet tab-stop session, RDiffView. L-REG: generate `src/lang/basic.rs`'s groups from the registry. I3: semantic tokens and outline folds on top. Width-aware columns (CJK, tabs) for vertical moves are the view's: the model counts one column per character.

### I0 / L-PARSE results (2026-10-06)

The parser for tools: trivia, the origin map, lexer recovery and the semantic model. About 2,600 lines (1,400 of code, 650 of tests) in the lane's crates and `interpreter/rapidr-bcgen`; no other crate touched. Commits: preprocessor, lexer, parser, bcgen, these notes.

**What landed.**
- **Origin map** (`rapidr-preprocessor`, `origin.rs`). `PreprocessResult.origins: OriginMap` maps every byte of the preprocessed text to (file, byte offset). Exact segments are the file's own bytes; generated ones (a `$DEFINE`'s value, a `$MACRO` expansion, `$RESOURCE` / `$OPTION ICON` constants, a built-in `RAPIDQ.INC`, the `$ESCAPECHARS` lines around an include) map to the source they replace. Substitutions go through a mapped text, so the map follows every edit; the text they produce is the same as before. It also keeps each file's decoded text, its encoding (`decode_source` / `encode_source` give the bytes back: UTF-8, BOM, Windows-1252) and what each line was to the preprocessor (`LineKind`: code, directive, inactive `$IFDEF` branch, `#!`). There are lookups both ways (`origin`, `origin_span`, `to_preprocessed`). `preprocess_*_recovering` reports a missing or malformed `$INCLUDE` / `$RESOURCE` on its line and goes on. `scan_lines` classifies one file's lines without reading its includes.
- **Lexer recovery** (`rapidr-lexer`). `Lexer::tokenize_recovering` turns unreadable text into `TokenType::Error` tokens (an unterminated string at the end of the file becomes the string as far as it goes), returns every error and goes on. `tokenize` is unchanged: it returns the first error, and the tokens are the same up to it.
- **Trivia** (`rapidr-lexer`, `lossless.rs`). `LosslessFile` lexes one file with the compiler's own lexer: same tokens, the AST untouched. Whitespace, `'` / `REM` comments, `_` continuations and the line breaks they join, preprocessor directive lines, inactive branches and `#!` go in a side table of `Trivia`. Tokens + trivia cover every byte once, in order, and `print()` gives the file back. Blank lines are their `Newline` token with only whitespace before it; `lines()` classifies each line as blank, comment, directive, inactive or code. Directives the parser reads (`$TYPECHECK`, `$APPTYPE`, `$OPTION`, `$ESCAPECHARS`) stay `Directive` tokens.
- **Parser for tools** (`rapidr-parser`, `tools.rs`). `parse_file_for_tools` / `parse_source_for_tools` run the compiler's preprocessor, lexer and parser (the backends' AST) and never stop at an error. `locate(span)` gives (file, byte range, line, column, exact) through `$INCLUDE`, `$DEFINE` and `$MACRO`. `files` holds every file of the program losslessly, lexed with this build's real `$IFDEF` decisions. `diagnostic_locations()` places every diagnostic in its file.
- **Semantic model** (`rapidr-bcgen`, `semantic.rs`). `analyze(program, source)` compiles with a recorder and returns:
  - scopes: the program, SUBs / FUNCTIONs, TYPEs and their methods;
  - symbols: globals, locals, parameters, STATICs, constants, components, routines, DLL routines, TYPEs, fields and labels, with declaration spans and types (declared, suffix or implicit; components under their R name, one model for Q and R);
  - references, each marked declare, read, write or call;
  - lookups for I3: `symbol_at`, `scope_at`, `lookup`, `visible`.

  bcgen now calls the model's rules itself, so the compiler and the IDE can't disagree:
  - `resolve_name`: a bare name is a component, then a local, then a bare builtin, `True` / `False`, a RapidR constant or a FUNCTION, then a global;
  - `dim_target`, `for_target` and `store_target`, for DIM, FOR and assignments.

  Names handled through other compiler paths (`Obj.Member`'s object, a routine called by name) are looked up in the same tables, innermost scope first. `RESULT` in a FUNCTION refers to the FUNCTION.

**No change to what any program means.** A fingerprint of every compiler stage covered 675 programs (the corpus' 386 examples and the repo's 289 .bas / .rr files), with and without RapidQ's `include` directory on the path. The stages: `preprocess_source` (the web's path), `preprocess_file`, tokens, the recovering parse, bytecode (`Module::to_bytes` + warnings) and the generated Rust. All are byte-identical between the base commit and the lane's head; with the includes, 603 programs reach bytecode and Rust. Two compiler hangs end, both on programs that never compiled:
- an empty `$MACRO` / `#Const` name;
- a macro that expands to itself (now at most 10,000 expansions per line).

The suites, with the lane's `./rapidr` and web build:
- conformance on both backends: 312 passed, 0 failed;
- web conformance: 135 passed, 3 known failures (xfail), 0 failed, 0 page errors;
- the desktop GUI events suite (native + interpreted): all 702 checks passed;
- `tools/rapidq_corpus.py`: 163 / 386 programs compile, 130 / 130 portable (100%).

`tools/corpus_compare.mjs` (each program run native vs interpreted) wasn't rerun. The disk was 99% full with the parallel lanes' builds, and its outcome can't change: both backends' inputs (the bytecode and the generated Rust) are byte-identical to the base for every corpus program.

**Acceptance.**
- Every RapidQ example of the corpus (386 `.bas` plus the corpus' 250 `.inc`) and every repo `.bas` / `.rr` / `.inc` (291) parses with trivia and round-trips byte for byte, to the encoded bytes (`crates/rapidr-parser/tests/tools_corpus.rs`). The same test checks every token span that maps exactly against its file's text: 790,000 tokens, including those in included files.
- Spans in a program with `$INCLUDE`s point into the right file at the right byte: `spans_point_into_included_files` (nested include directories, CRLF, `$DEFINE` in an include, a missing include reported on its line); unit tests in the preprocessor; the semantic corpus test. That test checks 217,000 references on the 675 programs, 31,000 of them in included files: each holds its name, in the right file. It also confirms that no compiler panic happens while recording.
- Fuzzing: `crates/rapidr-parser/tests/fuzz_edits.rs` makes random edits of real programs (deletions, snippets of directives, strings, continuations, non-ASCII, NUL, truncation) and runs them through the lossless lexer, the recovering preprocessor, lexer and parser, the origin map and the compiler's own entry points. Nothing may panic or hang (a 20 s watchdog per case). It runs 400 cases by default (about 1 s in debug), in regress's `unit` stage with the rest of `cargo test --workspace`. 100,000 cases in release and 20,000 in debug (overflow checks) ran clean. `RAPIDR_FUZZ_CASES` / `RAPIDR_FUZZ_SEED` reproduce a case.

**Shared files touched.** `Cargo.lock` (the parser's new dependency on `rapidr-preprocessor`, one line); this section of the plan. Nothing in `rapidr-ast`, `regress.sh` (the fuzz test's short run comes with `cargo test --workspace`), the root `Cargo.toml`, the ROADMAP or the CHANGELOG.

**For later stages.**
- I3: build on `ToolsParse` + `semantic::analyze`. Keep requests answered from the last model, and re-run `analyze` per edit: it's a compile, milliseconds for corpus-sized programs.
- I4: find CREATE blocks by the AST's spans and `locate`, and keep comments with `LosslessFile::leading_trivia`.
- Line kinds come from a file's first inclusion. A file included twice under different `$DEFINE`s shows its first decisions.
- `$IFDEF RAPIDR` (D12) isn't predefined yet. It is a preprocessor change, but a language change for programs that use the name `RAPIDR`, so it needs its own commit.

### I0 / L-SESS results (2026-10-06)

The project file, the program session protocol with both of its ends, and the VM's debugging primitives (the plan's L-VM items for I0, done in this lane).

**`rapidr-project`** (`crates/rapidr-project`, wasm-clean, 24 tests). Decision D2 as recommended: `.rrproj` format 2 in TOML — `format`, `name`, `main`, `[[files]]` (`path`, `kind`: module, form, include, resource, asset, data), `[build]`, `[run]`, `[compat]`, `[designer]`, `[ai]` with the data model's fields and defaults; every section optional, unknown keys ignored, format 1 / 3 refused with a reason. It is the RProject model (`Name`, `MainFile`, `FileCount`, `File(i)`, `CompatMode`; `Open`, `Save`, `New(Template)` with `console` and `gui` templates, `AddFile`, `RemoveFile`). Per-user state (open documents, dock layout, breakpoints, watches) in `.rapidr/workspace.toml`, the folder ignored by VCS (`.rapidr/.gitignore`). The web IDE's JSON v1 projects import (`import_v1`): each form becomes `<Form>.rr` holding its CREATE block written exactly as `model.js` wrote it (property order kept, `basicString`'s quoting, handlers as `OnClick = …` lines in the blocks) followed by its code; each module a `.rr`; inline data-URL assets files under `assets/`; a main `<project>.rr` that `$INCLUDE`s them in v1's order and shows the startup form. One known difference: a form's top-level statements now run after its own CREATE block rather than after every form's (v1 put all blocks first). Plain `.bas` / `.rr` files open as an implicit project that follows their `$INCLUDE`s; `open(path)` dispatches. Everything over strings is a pure function (the web); the file-system functions take paths.

**The VM's debugging primitives** (`rapidr-vm`, `rapidr-bcgen`, `rapidr-bytecode`; tests in bcgen):
- Breakpoints by file through the source map: `set_file_breakpoints(module, file, lines)` (a file is its name, as the source map keeps names only; a line without code moves to the next line of the same file with code; the answer says where each landed). Breakpoints now stop only at a statement's start, so returning from a call into the middle of a line with a breakpoint doesn't stop again. `SourceMap::compiled_lines` / `file_index`, `Module::code_lines`, `Function::line_at` (binary search).
- Pause on demand: `Vm::interrupt` (an `Arc<AtomicBool>`, settable from any thread or message handler), checked per instruction in debug mode only — non-debug runs pay nothing new.
- Evaluation: `rapidr_bcgen::compile_snippet` compiles statements against a stopped frame's symbols, rebuilt from the module (functions, parameters, local slot names; a global's spelling picked by which string slot holds a value); `Vm::evaluate` runs it on top of the stopped program with the frame's locals (written back on request), a fuel limit (`EVAL_FUEL`, 5 M instructions: `DO: LOOP` in a watch stops), and no stops, END, waits or yields. Limits for I6: a TYPE's fields by name and numeric conversions on store need the whole program's declarations, which the module doesn't carry.
- Writing a variable: `set_local`, `set_global`, and the protocol's `setVariable` (a snippet `name = (value)`, so array elements and properties work too).
- Break on runtime error: `break_on_error` stops at the faulting statement with every frame intact (`StopReason::Exception`, `stop_error`); going on, the error unwinds exactly as it would have (the innermost host-run handler fails, nested ones fail their caller, the main program ends) — `Frame::stack_base` makes that exact.
- A `Debugger` hook serves stops in place for hosts that can block (the desktop); hosts that can't (the web) get `VmError::Paused` as before. `frame_location` reports a caller's line as the line of its call.

**`rapidr-session`** (`crates/rapidr-session`; protocol + client wasm-clean, `program` behind a feature):
- The protocol (`protocol.rs`): requests `start`, `stop`, `pause`, `setBreakpoints {file, breakpoints:[{line, condition, hit, log}]}`, `setBreakOnError`, `continue`, `stepIn`, `stepOver`, `stepOut`, `stackTrace`, `scopes`, `variables {ref, start, count}` (arrays page by 100, objects show fields), `evaluate {expr, frame, context}` (`repl`: `? expr` evaluates, anything else runs as statements), `setVariable`, `setProperty`, `properties` (a component's properties as the runtime holds them; added for the web IDE's variables view), `input {text | form, event}`; events `ready`, `stopped {reason, file, line, description}`, `continued`, `output {stream, text}`, `formShown`, `formFrame` (defined, sent from I5), `formClosed`, `exited {code}`; replies carry `re` = the request's `seq` (`ok`, `error` or the request's own kind). JSON; condition / hit / log are carried, evaluated from I6.
- The program's end (`program.rs`): `ProgramEnd` serves requests on any host's VM; `BlockingDebugger` is the desktop's `Debugger`.
- The IDE's end (`client.rs`): `ProgramSession` is RProgramSession's model (`Program`, `Args`, `Debug`, `SeparateWindows`, `State`, `CurrentFile`, `CurrentLine`; `Start`, `Stop`, `Pause`, `Continue`, `StepIn/Over/Out`, `SetBreakpoint(File, Line, Condition)`, `Evaluate`, `SetProperty`; `OnOutput`, `OnStopped`, `OnExit`, `OnFormShown`) over a `Transport`; `process.rs` is the desktop's (a child process; the program's END exits at once, so the client turns the process's end into `exited`). Wrapping the model as a BASIC component (registry, runtimes' dispatch) is left to I1's L-PANELS / shell work, where it's first used.

**Desktop: `rapidr run --session <file> [args]`.** Requests on stdin (one JSON object per line, read by a thread: `stop` ends the process, `input` feeds INPUT, the rest reach the VM through its interrupt); events on stdout framed as an OSC sequence (`ESC ] 7767 ; json BEL`), so the program's own writes to stdout share the stream (the client's `Deframer` splits them; stderr is output too). The program waits for `start`, so breakpoints are set first. `rapidr-vm-host-native` gained a `session` feature (only the `rapidr` CLI enables it: programs' own runners don't carry it) and three host hooks (PRINT's output, INPUT's source, forms shown / closed). While a program waits in the window system, requests reach it when it next runs code (an event, a timer) — a Pause then stops at the next handler.

**Web.** The web compile fills the source map as the CLI does (and marks `$INCLUDE`d lines as library lines, as the CLI does); `compile_files(main, files, assets)` compiles a project from in-memory files — the preprocessor gained `PreprocessOptions::virtual_files` (additive; L-PARSE's crate, one lookup before the file system). `rapidr-vm-host-web`'s `DebugSession` class and the `__rapidr_debug_*` messages are deleted: the preview frame serves the protocol (`session_open(bytes, program, sink)`, `session_request(json)`; `{ __rapidr_session: json }` both ways over its MessagePort, the program's console output as `output` events), and the web IDE's debugger (host.js) is a client of it — stack, scopes, variables, component properties and the watches (now evaluated by the VM, not by name lookup in JavaScript) come from requests. The debugger's views now insert program-supplied text as text, not markup.

**Tests.** `crates/rapidr-cli/tests/session.rs` (3, the desktop through `rapidr run --session`) and `tests/web_session.mjs` (20 checks, the web runtime) run the same cases: a breakpoint in an `$INCLUDE`d file stops there on both runtimes, the stack spans both files, evaluate / setVariable change the result, pause on demand stops a busy loop, break on error stops at the faulting line and then unwinds (exit 1). Unit tests: 3 in bcgen (primitives), 5 in rapidr-session (framing across every split point, JSON round trips, a blocking session end to end, the client's state), 1 in the preprocessor (virtual files). `tests/web_ide_debugger_test.mjs`, `tests/debug_e2e_flow.mjs` and `tests/debug_e2e_event_handling.mjs` pass through the protocol (the two e2e scripts were stale since the DOM host went: they click the kernel's canvas now, take `RAPIDR_URL`, and are in `tools/regress.sh web` with `tests/web_session.mjs`; `rapidr-session` and `rapidr-project` joined the stage's wasm check).

**Verification (2026-10-06, this worktree).** Conformance on the VM 156 / 156; web conformance 135 passed, 3 xfail, 0 failed; GUI events (native + interpreted) all passed; web GUI parity 210 passed, 0 failed (the known ≠ only: menus, themes, message_icons / message_dialogs, design_surface 1×); web accessibility 78 / 78; every `tools/regress.sh web` browser suite passes except `tests/web_ide_e2e.mjs`, which fails the same way on the web IDE as committed before this lane (`#examples` options not found: pre-existing). `cargo deny` licences and bans ok, `THIRD_PARTY_NOTICES.md` regenerated (toml 0.9 and its parts, MIT / Apache), the built `rapidr`'s notices ok. The web runtime's wasm grew by ~200 KB (serde_json and the session).

**Coordination notes.** The origin map L-PARSE is adding (byte-level) isn't needed here: breakpoints and stops work on lines through the existing line map (`SourceMap`), and switch to byte spans when the editor needs columns. `interpreter/rapidr-compiler-wasm` (unused by the tools) still compiles without a source map.

### I1 / L-DOCK results (2026-10-06)

**RDockManager**, the public docking component, built model first.

- **Model** — `crates/rapidr-value/src/dock/` (GUI-free, builds for wasm32): `mod.rs` the `DockLayout` of §4 I1 (`Node::Split { axis, children, sizes } | Tabs { panes, active } | Documents`, auto-hidden panes by edge, floating groups, the documents and their mode) with every operation (insert at an edge / beside a group or the documents / into a group / as a document / floating / auto-hidden; remove, remembering the `Place` to go back to; normalization) and the text format `SaveLayout` / `LoadLayout` read back exactly; `geometry.rs` (groups, headers, tabs, buttons, splitters, auto-hide strips and the flyout, the documents' tabs, the docking compass and its previews, hit tests — theme-free, so components sit where they sit in every look); `look.rs` (the four themes' chrome as the kernel's ops, the pane icons drawn as shapes); `manager.rs` (panes, the API, the user's actions, the keyboard's move); `runtime.rs` (one glue for both runtimes: groups are `RDOCKGROUP` components, the document area an `RDOCKDOCS` that is an MDI client through `rapidr_value::mdi` or the tabbed page, the flyout an `RDOCKGROUP` stacked on top, a floating group an `RFORM` of its own — a top-level window on the desktop, a kernel window on the web — then OnPaneChange / OnDocumentActivate / OnDocumentClose (Cancel) / OnLayoutChange); `access.rs` (the screen reader's nodes). 21 unit tests (every operation, the round trip, geometry, compass, hide / show / auto-hide / float / dock back / drops / documents / splitters / reset / load, the keyboard's move, F6's order, every theme's ops, the accessibility nodes).
- **Kernel** — `crates/rapidr-ui-kernel/src/components/dock.rs`: RDOCKMANAGER (ground, splitters, strips; the compass, its outline and label and the flyout's shadow drawn over everything), RDOCKGROUP (header, tabs, buttons; press, drag to the compass, double click to float / dock), RDOCKDOCS; the keys (F6 / Shift+F6, Ctrl+Tab / Ctrl+Shift+Tab, Ctrl+Shift+M then arrows / Tab / Space / Enter / F / Escape; Escape slides a flyout in) before the focused component. The user's actions reach the runtime as `Container::Dock`.
- **Runtimes** — `crates/rapidr-runtime-core/src/dock.rs`, `crates/rapidr-runtime-web/src/dock_web.rs` (each ~70 lines over the shared glue). Pane names come back lowercase on both backends (a native build's component arguments arrive lowercase).
- **API** as built: docs/ide-components.md §3.7 (AddPane's `Where` grew `<side>:<pane>`, `<side>:documents`, `autohide:<side>`; added ClosePane, DockPane, MovePane, Pane / Document / PaneTitle / PaneState / PaneVisible, Next / PreviousDocument, ActivePane, DocumentCount, Layout).
- **Tests** — GUI case `dock_manager` (`tests/fixtures/dock_manager.bas`): a tab click, a splitter drag, the strip's flyout in and out, a drag onto the compass, the keyboard's move, F6, SaveLayout → changes → LoadLayout giving the same text, tabbed documents closed with OnDocumentClose's Cancel, a pane floated. Native and interpreted agree at 1× and 2×; on the web's kernel host every expected line, the accessibility tree equal to the desktop's, both windows byte-identical at 2× and the floating window at 1× (the main window at 1×: one pixel of the log label's text differs by one level — the known wasm text rounding, outside the dock). `kinds_agree_with_the_shared_rules` passes with the three kinds.
- **Looked at** — every state (docked with an active pane, flyout, compass while dragging, tabbed documents, floating, MDI cascade, MDI tile, the keyboard's move) in classic, modern, dark and highcontrast at 1× and 2× (`scratch/dock/gallery.bas`, 72 captures), and in real macOS windows (a real drag floating Properties into a top-level window).
- **Shared files touched** (one or a few lines each): `rapidr-value/src/lib.rs` (`pub mod dock`), `objects/a11y.rs` (RDOCKGROUP / RDOCKDOCS are groups), `component_defaults.rs`, `layout.rs` (default size), `members.rs` (SaveLayout … read without parentheses); `rapidr-ast/src/lib.rs` (COMPONENT_TYPES); kernel `components/mod.rs` (KINDS ×3), `components/form.rs` (`Container::Dock`), `tree.rs` (the flyout stacked last), `input.rs` (the dock's keys); runtime-core `lib.rs`, `object.rs` (method / get / set / after-set hooks, is_component_type), `mdi.rs` (the document area's MDI frames report to the dock), `ui/program.rs` (`Container::Dock`); runtime-web `lib.rs`, `object_web.rs`, `mdi_web.rs`, `kernel_web.rs` (the same); `tests/gui_parity_cases.mjs`; `tools/manual_reference.py` + `docs/manual/reference/components.md` (RDOCKMANAGER under Forms and containers). Not touched: ROADMAP, CHANGELOG, regress.sh, Cargo.toml. For L-REG's registry: RDOCKMANAGER (RapidR's own, no RapidQ name), internal kinds RDOCKGROUP / RDOCKDOCS (not program types).
- **Open** — (1) on macOS a window shown from a mouse handler opens behind the window clicked (any form: `Form2.Show` from a click too), so a floated pane can end up behind the main window: floating panes want an owned / tool-window level in the winit host (host lane). (2) Dragging a floating window's group back onto the main window's compass (across windows) isn't there; it docks back by its dock button, a double click on its header, or ShowPane after DockPane. (3) Tabs that don't fit shrink (no overflow menu yet). (4) Icons are the built-in shape set; picture icons come with D8's icon set (L-SHELL). (5) The compass's target is shown and labelled but not announced to screen readers yet.

### Icons (D8) results — 2026-10-06

RapidR's own icon set is done: every action, component, file type, symbol, toolbox group and glyph, our own drawings (MIT), one family with the brand (design/brand). The design system is [design/icons/README.md](../design/icons/README.md).

- **344 icons**, each drawn once on a 24 px master grid (2 px padding, 1.5 px round strokes). A geometry kit of our own (`design/icons/tools`, Python standard library) hints each to 16, 24 and 32 px:
  - 16 px: 1 px strokes, crisp;
  - 24 px: 1.5 px strokes, crisp at 2×;
  - 32 px: 2 px strokes, crisp.

  Small sizes are simplified where they need to be.

  | Category | Icons |
  |---|---|
  | Actions | 121 |
  | Glyphs | 22 |
  | Components | 145: the 96 types in COMPONENT_TYPES (RPROGRESS shares RPROGRESSBAR's), 42 planned components of this plan (IDE panels, I7's data components, RAI) and 8 chart kinds |
  | Files | 22 |
  | Symbols | 17 |
  | Toolbox groups | 17 |
- **Colour.** Monochrome action icons are in `currentColor`. Component, file, symbol and group icons are two-tone: a neutral frame plus one hue, written as colour tokens. Every theme has a palette:
  - classic and modern share the light one;
  - dark has its own;
  - high contrast is monochrome.

  Every hue has 3:1 against each theme's backgrounds and its own tint; a test checks it.
- **The inventory** is `design/icons/inventory.toml`:
  - components come from `rapidr_ast::COMPONENT_TYPES`, and from rapidr-lang's registry once it lands;
  - it also holds the 116 commands of I1–I6, the markers, file kinds, project kinds, symbol kinds and the toolbox groups;
  - the groups keep RapidQ's components (RC.EXE's and its include libraries' names) under "RapidQ" and the rest under "RapidR".

  The build and the crate's tests fail when a component type or a command has no icon.
- **Pipeline.** `design/icons/tools/build.py` writes the sources (`design/icons/src`), optimizes and checks them, and generates `crates/rapidr-icons`. The new crate, `rapidr-icons`:
  - themes the icons;
  - picks the drawing hinted for the device size (16 at 1.5× is the 24 drawing, at 2× the 32 one);
  - renders through resvg, and builds for wasm32;
  - stores the SVGs deflated: 459 KB of SVG become 42 KB, inflated on first use.

  Measured cost (rapidr-value's feature `icons` on vs off):

  | Build | Cost |
  |---|---|
  | Native console hello world | +224 bytes (the linker drops the icons) |
  | Native GUI hello world | +154 KB |
  | Web runtime wasm | +119 KB raw, +64 KB brotli (2.71 → 2.78 MB) |

  The UI kernel draws icons with `Painter::icon` (`crates/rapidr-ui-kernel/src/icons.rs`), identically on both hosts.
- **Public API (additive).** `Bitmap.LoadIcon(Name$ [, Size [, Theme$]])` and `ImageList.AddIcon(Name$ [, Theme$])` (`rapidr_value::objects::icons`, rapidr-value's default feature `icons`). They take an icon name, a component type (QBUTTON) or a command id, so RToolBar's `AddButton(Name, Icon, …)` and RDockManager's `AddPane(…, Icon)` can take the same names. The mock-ups use this API.
- **Docs.**
  - `design/icons/tools/export.py` writes `docs/manual/icons/`: every icon as SVG (light and dark), as PNG, and an HTML catalog with a filter.
  - The manual's component reference (`tools/manual_reference.py`) shows each component's icon.
  - `tools/regress.sh unit` checks that both are current.
- **Review.** `design/icons/review/`:
  - contact sheets per category at 16, 24 and 32 px, in the four themes, at 1× and 2× (rendered by the crate);
  - mock-ups drawn by the UI kernel in every theme at 1× and 2×: a toolbar of flat QCOOLBTNs, a toolbox column and a problems / breakpoints list.
- **Found on the way.** resvg's rasterizer (tiny-skia 0.12) strokes a path's implicit closing segment, and unrounded `<rect>` outlines, half a pixel soft. The kit always closes paths explicitly and draws rects as paths. `small_sizes_are_crisp` checks it, and the visual-fidelity lane was told.
- **Changed in the plan.** I1's "vector icons (a permissive set — Lucide, ISC, or Tabler, MIT — plus our own component icons)" is now this set. The VS Code extension and the website take their icons from `docs/manual/icons/` (or `rapidr_icons::themed_svg`). Turning the file icons into a VS Code file-icon theme is a small follow-up for the extension's owner.

### VS Code extension (L-LSP) results — 2026-10-06

**What landed.** `rapidr-langsvc` (the I3 API: `Analysis::new / update / completions / hover / signature / definition / references / prepare_rename / rename / diagnostics / outline / semantic_tokens / format / code_actions`; wasm-safe, ~4,500 lines with `rapidr-lsp`), **`rapidr lsp`** (lsp-server 0.10 + lsp-types 0.97, synchronous; UTF-16 or UTF-8 positions; diagnostics published once no message waits), **`rapidr dap`** (`rapidr-dap`, ~2,800 lines: our own DAP types, mapped onto the session protocol), and the **VS Code extension** rebuilt as a thin client (the six regex providers deleted; 632 lines of JS: LSP client, debug adapter descriptor, locating `rapidr`, Run / Debug / Build Native / Build Standalone / Bundle for the Web, a status bar item), `rapidr-2.117.0.vsix` (1.3 MB with its screenshots) built by `tools/release/vscode.sh` into the release, publishing by hand ([docs/vscode-publishing.md](vscode-publishing.md)).

**Answers.** Completion of the program's names in scope (locals, parameters, globals, RapidQ's implicit globals, SUBs, FUNCTIONs, TYPEs, constants, components), components' members by the variable's type — after `Obj.`, after a WITH's `.`, in a CREATE body (properties and events as `Name = `), through `TYPE … EXTENDS` (fields of the base TYPE, members of the base component), `This` / `Super` —, RapidQ's components under Q names after `AS` and RapidR's own under R names, builtins, statements, directives, labels after GOTO. Hover (declaration, kind, type, Q / R names, where declared, how often used), signature help (parentheses, and SUB calls without them), definition (also of an `$INCLUDE` line), references and rename from the model's references (rename refuses invalid names, reserved words and any clash in a scope that uses the name), outline (the CREATE tree, TYPE members), semantic tokens, a re-indenting formatter that never changes a token, and RapidQ-compatibility warnings with a quick fix (`QPLOT` → `RPlot`; RapidR-only components).

**Diagnostics are the compiler's.** The service runs the pipeline `rapidr build-bc` runs, on the editor's text: same messages, same line and column (the range runs from the compiler's column over the name the message is about). Checked against `rapidr build-bc` itself for every conformance error case (`crates/rapidr-cli/tests/lsp.rs`). The bytecode compiler's errors are text (`LINE:COL: error: …`), read in one place (`diagnostics::compiler_errors`) — **patch wanted in rapidr-bcgen**: its `errors` as `Diagnostic`s with spans, then that function goes.

**The seams (what changes when the foundations land).**
- **L-PARSE's parser for tools** → `crates/rapidr-langsvc/src/front.rs` only: `parse()` calls `rapidr_parser::tools::parse_source_for_tools`, `Parsed::locate` / `to_preprocessed` forward to `ToolsParse::locate` / `to_preprocessed`. Until then the module recovers by blanking the line in error and maps positions line by line through the preprocessor's line map (exact on lines the preprocessor copies; best effort on `$DEFINE` / `$MACRO` rewrites; nothing located on a directive's generated code). `$INCLUDE`d files are read from the disk by the preprocessor (editor overlays come with L-PARSE).
- **L-PARSE's semantic model** → `crates/rapidr-langsvc/src/model.rs` becomes `pub use rapidr_bcgen::semantic::*;` — its types (`SemanticModel`, `Symbol`, `SymbolKind`, `Scope`, `ScopeKind`, `Reference`, `Access`, `symbol_at` / `references_to` / `scope_at` / `lookup` / `visible`, `name_key`) are copied exactly; the interim `analyze` walks the AST with the compiler's rules (a component, then a local, then a global; DIM in the main program global, in a SUB local; implicit globals).
- **L-REG's registry** → `crates/rapidr-langsvc/src/registry/`: its types and functions are a subset of `rapidr-lang`'s with the same names and fields (`component`, `builtin`, `builtin_key`, `constant`, `Component::{property, method, event, written_name}`, `Param::text` …); `data.rs` (generated once from the legacy JS tables, member spellings from the corpus, a short glossary in our own words) is deleted. Then the compatibility checks extend to members, builtins, statements and directives (each one's origin), and hovers get the registry's docs.
- **L-SESS's `rapidr run --session`** → `crates/rapidr-dap/src/lib.rs::program_end_command` (`__debuggee` → `run --session`), `session_wire.rs` → `pub use rapidr_session::protocol::*;`, delete `src/interim/` and the CLI's `__debuggee` route. The adapter doesn't change; L-SESS's evaluator then answers what the interim end refuses (expressions, conditions, logpoints) and break-on-error inside event handlers keeps its frames.

**Tests.** `tests/langsvc/` golden suite (10 cases, markers `|`, expectations as `'!` comments: completion / hover / definition / references / signature / rename / outline / diagnostics — CREATE bodies, WITH, TYPE methods and EXTENDS, `$INCLUDE` files, Q and R names, RapidQ's implicit scope, RAPIDQ.INC); every conformance error case's diagnostics (in-crate, and against `rapidr build-bc` over LSP); robustness: 569 programs (examples, conformance, RapidQ's corpus) asked every request at 25 places, whole and cut short, no panic (20 s release); LSP protocol tests over stdio (3); DAP protocol tests (6: breakpoints in two files, the stack, variables, stepping, pause, break on error with INPUT, a breakpoint in a GUI handler and in a modal form's timer); extension: 15 unit + 18 integration tests in a real VS Code (test-electron, throwaway profile) against the real `rapidr`, including a debug session. Screenshots: `npm run screenshots` (macOS) drives VS Code through each feature and writes `utilities/vscodeext/rapidr/images/*.png`.

**Found on the way (not this lane's to change).** (1) `Obj.` alone at the end of a line joins the next line: `Form.` then `Nope x` compiles as a member call (`rapidr build-bc` accepts it) — the parser's member access crosses the line end. (2) With `$TYPECHECK ON`, an undeclared name as a method's argument (`List.AddItems itme`) isn't reported; an assignment's is. (3) The DAP lane saw a QTIMER never tick when MAIN ends with `Form.Show` (not ShowModal), with or without the debugger.

### I0 / L-REG results — the language registry (2026-10-06)

**What landed.** `crates/rapidr-lang` (new workspace member): the registry's data in `data/*.toml` (~9,000 lines: `components/<family>.toml`, `globals.toml`, `items.toml`, `sets.toml`, `glossary/{properties,methods,events}.toml`, `builtins.toml`, `language.toml`, `constants.toml`), compiled by `build.rs` into static tables (decision D5) and checked as it builds — names unique, types known, enum values and constant defaults real constants, parameter lists well formed, default events real events, sets and item kinds defined; a mistake fails the build naming the file and entry. No runtime dependencies (`toml` 1.x only as a build dependency, already in the tree), builds for `wasm32-unknown-unknown` (in `tools/regress.sh unit`'s wasm check). `src/lib.rs` documents the format.

| What | How many |
|---|---|
| Components the compilers create (`COMPONENT_TYPES`) | 97 — 74 with a RapidQ name (and its aliases: QGAUGE, QOUTLINE, COMPORT), 23 RapidR's own |
| Other components | 2 RapidR BASIC libraries (QDOCKFORM, QDIRLISTVIEW); 6 RapidQ objects not built yet: QOLECONTAINER, QOLEOBJECT and — found in RC.EXE's own table, in none of RapidR's lists before — QTHREAD, QTRANSIMAGE, QD3DANIMATION, QD3DANIMATIONSET |
| Global objects / item objects | 6 (Screen, Application, Clipboard, Mouse, FileRec, Printer) / 5 (TreeNode, HeaderSection, StatusPanel, ListItem, ListColumn: what `Item(i)`, `Sections(i)`, `Panel(i)`, `Column(i)` give) |
| Members | 3,459: 1,972 properties, 1,200 methods, 287 events; 2,080 RapidQ's, 1,379 RapidR extensions; 101 RapidQ members RapidR doesn't answer yet (`missing`); 132 one runtime only (`only`) |
| Builtins / statements / directives / keywords / type names | 158 (+ 67 internal names of BUILTINS) / 62 / 15 / 32 / 14 |
| Constants | 516 in 108 groups (RAPIDQ.INC, the library includes', RapidR's anchors) |
| Docs | every entry, in RapidR's own words (1,121 glossary entries by name, the rest per entry); categories for every property |

Each entry carries origin (RapidQ / RapidR, and the include file for RapidQ library members), RapidQ and RapidR names, types (and enum values, component kinds), defaults (RapidQ's), read-only / write-only / design-time flags, categories, parameter lists and return types, the value-method flag (`IF Dlg.Execute THEN`), the default event, the runtimes that answer it.

**Seeded from the code's truth and RapidQ's facts** (`tools/lang_seed.py`, one-time; the TOML is the source now): `COMPONENT_TYPES` and the Q-name rules, the runtimes' dispatch (`tools/lang_dispatch.py`), the VM (every method called once — which answer, block or warn; every property's first value), the web IDE's old data (only names the runtimes' sources use; its descriptions, RapidR's words), RapidQ's manual tables (names, types, R/W, defaults, parameters — never its text), KEYWORD.LST, and RC.EXE itself: its class/member string table (70 classes, 1,740 member pairs) and ~730 compile probes in the Windows VM (members, builtins with `$TYPECHECK ON`, statements, global objects) — the probes and the table agreed on all 620 members both judged. Four agents wrote the docs in parallel (properties, methods, events + component docs, builtins + statements + constants) and reported data errors, which were fixed.

**The generation switch.** `rapidr_ast::COMPONENT_TYPES` is `rapidr_lang::COMPONENT_TYPES`; the desktop and web dispatchers' `is_component_type` / `is_component_method` are `rapidr_lang`'s (their hand lists — 450 lines — deleted; no program called them, so nothing changes). `rapidr-editor`'s BASIC keyword groups (L-EDCORE's copied lists, which its results handed to L-REG) are `rapidr_lang::words`.

**Generators** (`rapidr lang export`): `--json` (the IDE's completion data: everything above), `--prompt` (the AI system prompt's language section), `--vscode` (`utilities/vscodeext/rapidr/src/languageData.js`, same shape, the extension unchanged), `--web-ide` (`web-ide/lang-data.js` until `web-ide/` goes; every component also under its Q names, as docs/q-and-r-components.md §4 asks of completion), `--manual` (docs/manual/reference: components, **members** (new: every component's properties, methods and events), builtins, **statements** (new), **constants** (new), data science), `--all` (every generated file; `RAPIDR_LANG_BLESS=1 cargo test -p rapidr-lang --test generated` does the same). `tools/manual_reference.py` is gone. `rapidr lang conformance DIR` writes the conformance programs.

**Tests that tie it to reality.**
- `cargo test -p rapidr-lang` (21 tests): `tests/coverage.rs` — `COMPONENT_TYPES` and `canonical_type_name` / `LIBRARY_TYPES` / `INCLUDE_LIBRARY_COMPONENTS` / `RAPIDR_LIBRARIES` / the not-yet list agree with the registry; every BUILTINS name is a builtin or an internal name, bare ones are `bare`, `missing` ones aren't in BUILTINS, the rest compile; bcgen's RapidQ-builtins list is the registry's RapidQ builtins; every lexer keyword and every word the parser recognizes is a statement, keyword, type or builtin, and every statement is one they know; the preprocessor's directives both ways; RAPIDQ.INC's constants in order, the library includes' and RapidR's; every method's `value` is `rapidr_value::members`' rule; every entry has docs; categories known. `tests/generated.rs` — the generated files are current.
- The reverse check, `tools/lang_dispatch.py --check` (in `regress.sh unit`): every `match` on a member name in the runtimes (138 sites in rapidr-value, runtime-core and runtime-web, ~1,440 names, plus `("RTYPE", "member")` tuple arms) is mapped to the components it serves, and every name it answers is a member of one of them in the registry (a dozen desktop no-op stubs listed with their reasons). It caught development's new data-science model as soon as it was merged.
- `tests/lang_conformance.mjs` (in `regress.sh` conformance — `vm,native` — and web): a generated program per component reads every readable property (printing it where the registry has a default), calls every method (arguments from its parameters; `test = "skip: …"` where it waits for the user or needs a scene) and binds a handler to every event; each runtime must print the registry's output and answer every member (no "not implemented"). **Interpreter 88 / 88, native 88 / 88 (one build), web 95 / 95** (the web-only components too). The values RapidR doesn't give yet are in `tests/lang/gaps.txt` (252 for every runtime, 11 `web:`), which `--gaps` lists.

**What it found** (reported, not fixed here — this lane changes no program's behaviour):
1. **252 RapidQ defaults RapidR doesn't give at creation**: Align, Cursor, ShowHint, Kind, ModalResult, Spacing, … read empty until set (RapidQ: alNone = 0, crDefault = 0, False, …); e.g. `QBUTTON.Cursor`, `QLABEL.Enabled`, `QFORM.KeyPreview`. The web gives some of them (QCOOLBTN / QOVALBTN's Down, Flat, GroupIndex, AllowAllUp) where the desktop doesn't. Fixing them is `component_defaults`' step 2 (web host plan W3): RapidQ's value on every runtime, then the gaps file's lines go.
2. **101 RapidQ members RapidR doesn't answer**: QFORM's HideTitleBar / ShowTitleBar / ShapeForm / TextRect, QBUTTON's StartDrag, QMYSQL's CreateDB / DropDB / FetchLengths / RealConnect / RowBlob, the streams' CopyFrom / LoadArray / SaveArray / LoadUDTArray, QMEMORYSTREAM's MemCopyFrom / MemCopyTo, and 18 events nothing fires (OnHint, OnEnter, WndProc, OnStartDrag / OnEndDrag, QCOMPORT's OnBreak / OnRing / OnTxEmpty …). Each is `missing = true`, so the IDE and the compat diagnostics know.
3. **Desktop-only members**: drawing methods on lists, grids, bitmaps and QDXSCREEN, Clear on images / memory streams / status bars, a list view's AddItem / DeleteItem, Click / SetParent (the desktop's generic dispatch) — the web warns "not implemented" for them.
4. **`ReadAll` past a stream's start** (`rapidr_value::objects::memstream::read`: `pos + usize::MAX`) returns "" in release builds and panics in debug native builds; skipped in the programs with that reason.
5. bcgen's `RAPIDQ_BUILTINS` lists 12 names RC.EXE doesn't know (MICROTIMER, RTLMOVEMEMORY, GETCAPTURE, SETFOCUS, WSTRING …: include-file and Windows API functions); the registry groups them as "Library functions".

**Verification** (this worktree, after merging `development` at `b3822d3`): `cargo test --workspace` green (rapidr-lang 21 tests, rapidr-editor's 43 with the registry's groups); the conformance suite on both backends 326 / 326; `tests/native_gui_events.mjs` all checks passed; the web conformance 140 passed, 3 xfail, 0 failed; the registry's programs 88 / 88 interpreted, 88 / 88 native, 95 / 95 web; the web suites as before (`web_ide_e2e`'s bundle renders fail the same with development's own `lang-data.js`: not this change); `cargo deny` and `THIRD_PARTY_NOTICES.md` current.

**Shared files touched** (for the integrator): `Cargo.toml` (member, workspace dependency), `crates/rapidr-ast/src/lib.rs` (`COMPONENT_TYPES` = the registry's), `crates/rapidr-runtime-core/src/object.rs` and `crates/rapidr-runtime-web/src/object_web.rs` (the two predicates re-exported from rapidr-lang, hand lists deleted), `tools/regress.sh` (unit: rapidr-lang in the wasm check, `lang_dispatch.py --check` instead of `manual_reference.py --check`; conformance and web: `tests/lang_conformance.mjs`). Not touched: ROADMAP.md, CHANGELOG.md, objects/mod.rs.

**Left.** `component_defaults` generated from the registry once the default gaps are decided (a behaviour change: its own step); `rapidr_value::members`' value-method lists and bcgen's `RAPIDQ_BUILTINS` generated too (both are checked against the registry today); `web-ide/host.js`'s `EVENT_META` / `COLOR_PROPS` hand tables go with `web-ide/` (I1); I1's toolbox and inspector, I3's completion, hover and compat diagnostics, and I8's prompt read `rapidr_lang` (or `export::json`) directly.

**Second round (2026-10-06): on the foundations.** The seams above are closed — L-PARSE and L-SESS merged, L-REG still running:
- **front.rs** is the parser for tools (`parse_source_for_tools`: the recovering lexer, the byte-level origin map — positions through `$DEFINE` / `$MACRO` exact). The program's other open files go to the preprocessor as `PreprocessOptions::virtual_files`: an `$INCLUDE`d file edited and not saved is what completion, navigation and diagnostics see; requests in an include are answered by the open program that includes it.
- **model.rs** is `pub use rapidr_bcgen::semantic::*` (the interim walker is gone). The compiler's model gained, in `semantic.rs`: references to TYPE fields and methods through `Obj.Member`, `This`, WITH and fields of fields (rename and references of a field are exact); a TYPE extending another of the program's has it as parent scope; and a SUB's own undeclared variable — RC.EXE's implicit scope, which the compiler renames `Routine__name` — is that SUB's (hover: "variable of `Up` (implicit: its own, kept between calls)"), no longer merged with the main program's same name.
- **rapidr-bcgen's diagnostics are structured**: `compile_program_diagnostics` (span, line, column, message; `error_text` prints exactly what the CLI printed before), mapped by the origin map; the text parsing is deleted.
- **`rapidr dap` runs `rapidr run --session`**; `src/interim/` and the CLI's `__debuggee` are deleted, `session_wire` re-exports `rapidr_session::protocol`. Watches, hovers and the Variables view evaluate expressions in the frame on the VM; the debug console prints an expression and runs a statement (VB's Immediate); break on error inside an event handler keeps its frames and locals. `compile_snippet` names a routine's own implicit variables as the compiler does (a watch of `p` in the SUB that made it). Paths stay as the editor wrote them (a `/var` / `/private/var` pair opened the file twice).
- Tests added: `crates/rapidr-langsvc/tests/open_files.rs` (an unsaved include; names after a `$DEFINE`), the implicit-scope golden case on RC.EXE's rule, LSP over stdio with an unsaved include, DAP expressions (watch / repl / hover) and a run-time error inside an event handler; extension: 19 integration tests (watch expressions and the console in the debug test; an unsaved include). Robustness (569 programs) and the bcgen corpus model test pass; VM conformance unchanged.
- Found: the session's Globals scope lists a SUB's own variables under the compiler's names (`S__p`) — rapidr-session's `program.rs` should show them as `p` under that SUB's frame (L-SESS).

**Third round (2026-10-06): the registry seam; every seam closed.** The language service and the VS Code extension now share RapidR Studio's language model end to end — parser for tools, the compiler's semantic model, the session protocol and the registry:
- **`crates/rapidr-langsvc/src/registry/` is deleted** (its 2,015-line `data.rs` and the interim statements table); completion, hover, signature help, outline, rename's reserved-word check and the compatibility diagnostics read `rapidr_lang` directly. rapidr-lang gained the lookups the service needed, for every client: `resolve_component` (a type name as the compilers read it: `QPLOT` is RPLOT — `rapidr_ast::canonical_type_name`'s rule), `statement` / `statement_starting` (`SELECT` → SELECT CASE), `keyword`, `directive`, `type_name`. Builtins have no `params` in the data, so signature help finds the parameters in the registry's `syntax` line (`MID$(String, Position, Num)`, `LOCATE [Y%][, X%][, cursor]`).
- **Registry-driven compatibility** (`compat.rs`, on the parser for tools' lossless tokens and the compiler's model): always — `missing` members, builtins and `planned` components ("RapidQ's QForm.ShapeForm is not implemented in RapidR yet", warning) and `only` members / builtins / components (a note: desktop only / web only); in a RapidQ-compatible project — every RapidR-origin component, member of a RapidQ component, builtin, statement (`OPEN`, `LINE INPUT`, `PRINT #`, `GLOBAL` …), directive (`$THEME`), type (`INT64`) and RapidR constant ("… is a RapidR extension: RapidQ's compiler refuses it"), plus the Q-name fix as before. A registry diagnostic where the compiler already reports one is dropped. Hovers carry the same flags plus a property's type, read-only / write-only, RapidQ's default and the include file; completion leaves out what's `missing`/`planned` and read-only properties in a CREATE body; global objects (`Screen.`) and item objects (`Tree.Item(0).`, from the property's `kinds`) resolve.
- **Globals show only globals** (rapidr-session `program.rs`, rapidr-bcgen `global_slot`): the one decoder of the names bcgen gives a routine's variables in global slots — STATICs (`SUB S::hits`, and their `#init` flag) and RapidQ's implicit routine variables (`S__p`) — so Globals lists the program's globals under their source spelling and a frame's Locals add its routine's own variables by their names. `compile_snippet` uses it too: a watch / the console / setVariable can read and write a STATIC in its frame (before, only the implicit ones). Shared by `rapidr dap` and the IDE (both serve the session through `ProgramEnd`).
- Also: a diagnostic whose focused word ends at its place (`Form.` at a line's end) underlined nothing; it now underlines the character there (`rapidr-cli` `tests/lsp.rs` `an_editor_session` expects the dot underlined, and failed without this).
- Tests: rapidr-session `a_routines_own_variables_are_its_frames_not_globals`; `rapidr-cli/tests/dap.rs` `a_subs_own_variables_are_its_locals_not_globals`; golden case `tests/langsvc/registry_compat` (`'! options rapidq-compatible`, new in golden.rs); rapidr-lang `language_words` and `resolve_component`; extension: 22 integration tests (+3: a SUB's own variables in Locals / Globals, a global object's hover, the not-implemented warning and the RapidQ-compatible switch), run in the installed VS Code 1.140 with a throwaway profile (`VSCODE_PATH`, new in `runTest.js` and `capture.js`) and again against the packaged `.vsix` installed into a throwaway extensions folder. `SHOTS_DIR=… node test/screenshots/capture.js` captures every state (the README's six, plus definition, rename, the registry's warnings, a member's hover and stepping a SUB with its own variables, `showcase/counter.bas`) into a folder instead of `images/`.
- Robustness (569 programs × 25 places): 22 s release, unchanged.
- **Automatic keyword case** (asked for by Robert in this round; `crates/rapidr-langsvc/src/case.rs`): `rapidr.keywordCase` (upper / lower / proper / preserve, default upper) and `rapidr.identifierCase` (declaration / preserve, default preserve), passed to `rapidr lsp` as initialization options (the client restarts the server when they change); `textDocument/onTypeFormatting` on space, Enter (the whole line left), Tab, `( ) , : = + - * / \ ^ & < > ;`, plus `rangeFormatting`; the extension turns `editor.formatOnType` on for `[rapidr]` (`configurationDefaults`). The formatter's indentation edits now cover only a line's leading and trailing blanks (so they never overlap a case edit), and leave alone a token that spans lines (found by the corpus test: `rapidq_literals.bas`'s `_`-continued string under `$ESCAPECHARS` had its second line re-indented — the string's own text). "Proper" is the first letter capitalised (`Byref`, `Elseif`): the registry has no mixed-case spellings of BASIC's words. **Programs keep their meaning**: `tests/case.rs` formats all 196 programs of `tests/conformance/cases` and `examples/` in three settings and compares the compiled bytecode, names compared without case (bcgen interns a name in the spelling it met first, and a component used as a value is its name as a string constant — `RANDOMIZE TIMER` with a QDXTIMER named `Timer` in `examples/directx/sprites.rr`), every string literal and comment byte for byte. Edge cases tested: mid-line `dim`, `Dimension` / `EndPoint`, `END SUB` / `End If`, `?`, CRLF, multi-byte strings, `$INCLUDE` paths, `Form.Caption` / `form.show`, a variable named `Left`, `Left = 12` in a CREATE body; in VS Code (typed a key at a time, completion off): one undo restores what was typed — 2 more integration tests (24, also run against the packaged .vsix installed in a throwaway profile); an LSP test of on-type and range formatting over CRLF. Screenshots: `case-typing.png`, `case-done.png` (`showcase/typing.bas`).

### I4 / L-DMODEL results — the designer model (2026-10-06)

**What landed** (~5,700 lines with tests). GUI-free, wasm-clean (`rapidr-designer` joins regress's wasm check; `rapidr-value` already builds for wasm32).

| Where | What |
|---|---|
| `rapidr_value::layout::engine` | *When* Align and Anchors place components — `after_set`, `client_changed`, `realign`, `reanchor`, `anchor_here` — over a `LayoutStore` trait. `runtime-core/layout.rs` and `runtime-web/layout_web.rs` are now stores of it (their copies of the sequence deleted); the designer is the third store. One implementation: the designer calls the runtimes' layout code. |
| `rapidr_value::designer` (the crate §3.1 names for models) | `model` (FormDesign: a CREATE tree — a node is a name, the type as written and RapidR's, and a body of assignments as source text, nested CREATEs and code it doesn't own, in source order; z-order = creation order), `command` (Command with its exact undo; `History`), `layout` (the CREATE blocks replayed through the engine: default sizes and Align from `layout::default_size` / `default_align`, Constraints, a QLABEL's AutoSize through `rapidr_value::autosize`, forms' frames and scroll bars through `scrollbars::Scroller`; `Layout::resize` is a user resizing the form), `snap` (the grid; guides for edges, centres, baselines, margins, equal spacing; 300 components well under 2 ms per move), `arrange` (align, distribute, same size, centre, nudge, z-order as CREATE blocks moved, Tab order by the runtimes' rule `a11y::tab_walk`), `inspect`, `text` (new components' CREATE text, Q names for RapidQ's components), `value` (values read with the program's constants), and `Designer` (model + selection + history + snapper; every operation one undoable step; a journal for the text side). |
| `crates/rapidr-designer` (new) | `Document`: a file's top-level CREATE blocks read with `rapidr_parser::tools` (spans through the origin map; the program's CONSTs resolve constants — `alClient` means 5 only where RAPIDQ.INC is included, as in RapidQ), each journaled command turned into the smallest `TextPatch` and the file read back (ids kept by name); one text history per file (`undo` / `redo` restore exact bytes); encodings kept (`open_bytes` / `bytes`). |
| RDESIGNSURFACE | `objects::design` on `Designer`: multi-select, rubber band, eight handles, guides drawn while dragging, round anchor pins (a click toggles), the form's corner drags the resize preview, one undo step per move; new members `Undo`, `Redo`, `AlignSelection`, `SelectAdd`, `SelCount`, `PreviewWidth` / `PreviewHeight`, `ShowGuides`, `SnapToGrid`, `GridSize` (in the registry). Chrome in `theme::current()`'s tokens (accent, window, border_strong, face, shadow): it follows the RAPIDR theme when that lands. The components' placeholder drawing stays (WYSIWYG is L-DVIEW's). Demo: `examples/form_designer.bas`. |

**APIs for the other lanes.**
- *L-PANELS (the inspected object)*: `Designer::inspect() -> Inspected { components, names, type_name, properties: [PropertyRow { name, ty, values, category, origin, doc, default, value, mixed, in_code }], events: [EventRow { name, params, origin, doc, handler, mixed }] }` — rows from the registry (design-time, read-write, not indexed; for several components the ones they share), values as the CREATE block writes them (`None`: the default). `Designer::set_property(prop, Some(typed) | None)` is one undoable command on every selected component (`inspect::format_value`: strings quoted, Booleans `True` / `False`); `inspect::set_value` gives the command without running it.
- *L-SHELL*: `Document::open_bytes(bytes, path, options)` → `forms()`, `designer(i)` (`&mut Designer`: `selection`, `add_component`, `delete`, `copy` / `cut` / `paste` (`Clip { trees, text }`), `duplicate`, `align` / `distribute` / `same_size` / `center_in_parent` / `nudge`, `bring_to_front` / `send_to_back`, `set_tab_order`, `place`, `set_anchors` / `toggle_anchor`, `rename`, `preview(w, h) -> Layout`, `layout()`), then `Document::sync() -> Vec<TextPatch>` (the edits for the code editor, in order), `undo` / `redo` / `can_undo` / `can_redo`; `set_text` when the editor changed the file; `diagnostics()`. A form designed without a file: `Designer::new(FormDesign::new(name, "QFORM"))`.

**Round trip** (`crates/rapidr-designer/tests/corpus.rs`). Every program with a CREATE block opens and, unchanged, writes back its exact bytes; every property set to its own value writes nothing; one component per form moved 8 px changes exactly one line's value, the text read back equals the model, the diagnostics don't change, and undo restores the bytes:

| | programs | with forms | forms | components | properties | moves | failures |
|---|---|---|---|---|---|---|---|
| `examples/` | 27 | 19 | 33 | 213 | 827 | 18 | 0 |
| RapidQ corpus (`~/Downloads/Rapidq/examples`, read only) | 386 | 245 | 348 | 2,444 | 10,445 | 183 | 0 |

Property tests: 200 random command sequences on the model (undo all = the start, redo all = the end, names unique, parents consistent); 60 sequences on a text with comments, `:`-joined lines and code inside CREATE (after each sync the text read back is the model; nothing outside the form changes; undo all = the original bytes).

**Anchoring equals the running program.** `tests/fixtures/designer_anchors.bas` (aligned panels, an AutoSize label, an edit, a memo, a panel and buttons anchored four ways, a nested anchored button, Constraints): `crates/rapidr-designer/tests/anchors.rs` resizes the designer's preview to 600 × 450 and must produce `tests/gui_parity_cases.mjs`'s `designer_anchors` expectations, which the program itself gives when resized by the user hook: native ✓, interpreted ✓ (`tests/native_gui_events.mjs`), browser ✓ at 1× and 2×. The designer's numbers and the runtimes' were computed independently and agreed on all 40 values.

**RC.EXE evidence** (the Windows VM, compiled from local copies, nothing printed): `Anchors = akRight + akBottom` in a CREATE and `Ok.Anchors = 12` → "Member ANCHORS not part of class OK"; `Constraints.MinWidth` → "Member CONSTRAINTS.MINWIDTH not part of class OK". Anchors and Constraints are RapidR's (Delphi's semantics, as `rapidr_value::layout` documents); the designer writes them with RapidR's built-in constants (`akLeft + akTop + akRight`), which need no include. Align, which RapidQ has, was run (a QSPLITTER alLeft, a QPANEL Width 200 alLeft, alTop / alBottom / alClient panels, an overflowing button). Findings for the compatibility lanes (not changed here — the designer follows the runtimes):
1. RapidQ aligns when the form's window exists (Delphi's `AlignControl` does nothing before the handle is allocated): a program whose CREATEs allocate no handle reads its aligned panels unaligned (185 wide, Top 0) until shown; once aligned, controls of one Align at equal positions go in creation order — RapidQ put the splitter (created first) at Left 0 and the panel at 3. RapidR aligns as each Align is set, the changed control first: the panel at 0, the splitter at 200 (layout.rs's comment says the opposite of RC.EXE here).
2. `Form.ClientWidth = 300` with a vertical scroll bar shown: RapidQ sets Width = 300 + (Width − ClientWidth) including the bar (Delphi's SetClientWidth), so ClientWidth reads 300; RapidR's `form_outer_size` ignores the bars (it reads 283).
3. RapidR's Tab order (`a11y::tab_walk`: by TabOrder, else creation index) isn't Delphi's (setting TabOrder inserts and renumbers the others); `designer::arrange::set_tab_order` writes TabOrder by RapidR's rule.

**Captures** (this worktree's `scratch/captures/`; desktop = the interpreted build with the test hooks, web = the browser's kernel host): `desk-preview-{1,2}x-1.png`, `web-preview-{1,2}x-1.png` (the form's corner dragged to 452 × 312: the memo anchored four ways shrinks, the side panel and OK / Cancel follow; pins on the selected memo), `desk-guides-{1,2}x-1.png`, `web-guides-{1,2}x-1.png` (Cancel dragged up: guides to OK's left, centre and right and to the label's top); each desktop / web pair byte-identical. Real input on macOS (`tools/real_input.py`: click, drag, Undo): `real-0-start.png` … `real-3-undone.png`.

**Shared files touched.** `Cargo.toml` (member, workspace dependency), `Cargo.lock`, `crates/rapidr-value/Cargo.toml` (rapidr-lang; proptest for tests), `crates/rapidr-value/src/lib.rs` (`pub mod designer`), `crates/rapidr-value/src/layout.rs` (`engine`, `anchor_set`), `crates/rapidr-value/src/objects/text.rs` (`line_metrics`), `crates/rapidr-value/src/objects/design.rs`, `crates/rapidr-ui-kernel/src/components/design.rs`, `crates/rapidr-runtime-core/src/layout.rs`, `crates/rapidr-runtime-web/src/layout_web.rs`, `crates/rapidr-lang/data/components/{display,forms,input,lists,datascience}.toml` (RDESIGNSURFACE's members; component sizes made the runtimes'), the generated `docs/manual/reference/members.md` and `web-ide/lang-data.js`, `tools/lang_dispatch.py` (sites), `tools/regress.sh` (wasm check), `tests/gui_parity_cases.mjs` (one case), `CHANGELOG.md`, this section. Not touched: KINDS, `object.rs`, `object_web.rs`, `objects/mod.rs`, `a11y.rs`, ROADMAP.

**Left** (L-DVIEW, L-SYNC, L-PANELS): WYSIWYG drawing of the real components; containers as drop targets on the surface (the model and the text side reparent already); keyboard placement with announcements; the menu and Tab-order editors; the live sync policy (debounce, read-only banner, "set in code" rows); rename through I3; a tab control's page area in the replay (its whole size today).

### I1 / L-SHELL + L-WEB results — RapidR Studio's shell (2026-10-06)

**RapidR Studio is one RapidR program** (`ide/studio.rr` and its includes, D3) on RapidR's public components and UI kernel: the same bytecode runs on the desktop (`rapidr ide [file]`, winit + vello) and in the browser (`tools/build_studio_web.sh` → `target/studio-web`, the kernel on a canvas).

- **The window.** A menu bar, a tool bar of the D8 icons (30 × 30 flat buttons with tooltips) and the command palette (Ctrl+Shift+P, `:N` goes to line N, examples too) are all built from one command table (`ide/commands.inc`: id, title, category, keys, icon, when enabled) with VB6 / Visual Studio keys (F5, Ctrl+F5, Shift+F5, F9, F8 / F10 / F11, Ctrl+S / O / N; Ctrl is ⌘ on a Mac). RDOCKMANAGER holds Project / Toolbox (left), Properties / Outline (right), Output / Problems / Immediate (bottom) round an MDI documents area (tabs one command away); F6 cycles the areas. A sectioned status bar turns green while the program runs, orange while it's paused. A Welcome page has the brand lockup, Start, Recent and the examples as cards in a grid that reflows.
- **What it does.** Opens a `.rrproj` (v2, or v1 imported), a source file with what it includes, or a folder; makes projects (File > New Project); saves files and the project; runs and stops the program through RPROGRAMSESSION — its own process on the desktop (its forms real windows), a sandboxed frame on the web — output into Output; outline and problems from RLANGUAGESERVICE as you type; Immediate evaluates in a paused program; theme (RapidR's look following the system by default, light / dark / high contrast, classic), MDI or tabs, the dock layout, recent projects and the default folder kept in Studio's settings (QREGISTRY, `HKCU\Software\RapidR\Studio`); `--do id,id` runs commands.
- **Services as components** (`crates/rapidr-studio`, both runtimes through a `Host` trait): RPROJECT, RLANGUAGESERVICE, RPROGRAMSESSION (docs/ide-components.md has them as built).
- **The web.** File System Access for files and folders (written back to the user's disk), `showDirectoryPicker` with a `webkitdirectory` fallback; Studio's own files kept in the browser's private file system (OPFS) between visits; F5 / F6 / Ctrl+Tab taken as the IDE's keys (`RAPIDR_APP_KEYS`).
- **The design pass** (Robert's review): Inter (UI) and JetBrains Mono (code) built in, OFL, Latin subsets ~72 KB each, RapidR Sans / MS Sans Serif only in the classic look, programs' defaults and the designer preview; 22 px tree rows, 28 px dock headers with actions on hover / focus, flat 30 px tool buttons, flat tabs with an accent line, MDI windows rounded with a light title bar, quieter separators; kernel tooltips (500 ms, under the mouse or the focused component, AccessKit description); one brand-blue accent on neutral greys, high contrast AAA. Studio's own dialogs (New Project, the palette) and the kernel-drawn ones (message / input boxes, colour and font dialogs) use the chrome font (`ide_theme::chrome_font`: Inter 10 pt in RapidR's look, MS Sans Serif in classic) with sizes to match.

**Captures** (`/Volumes/RapidRBuild/wt-a42bb200f3d369e5e/`): `studio-step1/` (step 1), `studio-pass1/` (after the design pass), `studio-before-after/`, `studio-dialogs/` (New Project, the palette, About; four themes, 1× and 2×), `studio-vm-real/` (real input: `macos/`, `windows/`, `ubuntu-wayland/`; `browsers/`: Edge, Chromium, Firefox). `tests/studio_shell.mjs`: 4 scenes (project, welcome, New Project, the palette) × 4 themes × 1× / 2× — **32 of 32 byte-identical desktop / web, accessibility trees equal**. `tests/studio_flows.mjs`: run with console output, outline and problems, theme and tabs, save a project, open a folder, the palette — 32 checks on both hosts.

**Every OS** (Parallels VMs, the source read from the share or the build volume; nothing printed):

| | Windows 11 ARM | Ubuntu 24.04 ARM | macOS |
|---|---|---|---|
| Build, `rapidr ide` | ✓ (MSVC, release) | ✓ (release) | ✓ |
| Headless captures vs the Mac's | 16 / 16 byte-identical | 16 / 16 byte-identical | — |
| Real input: F5 runs (program window opens), Shift+F5 stops | ✓ (`keybd_event`) | ✓ native Wayland (`ydotool`, kernel uinput) | ✓ (`tools/real_input.py`) |
| Palette → "RapidR Dark", Enter | ✓ | ✓ | ✓ |
| F6, Ctrl+S | ✓ | ✓ | ✓ (⌘S) |
| Example through the palette | ✓ | ✓ | ✓ |
| Alt+F4 closes, settings kept, reopens dark | ✓ | ✓ | — (⌘Q / close box) |
| Web page | Edge 154: FSA pickers and OPFS in a secure context; plain http → the fallbacks | Chromium 154 and Firefox 157 run Studio; Firefox: no FSA pickers → the `<input>` fallbacks | Playwright Chromium (the suites) |

Found and fixed by the real-input runs (all hosts): a window shown from a modal form took no input (Studio runs as `Studio.ShowModal`, its palette from there) — a modal form now blocks only the windows open when it went up, as Windows does; keys still held when a window got the focus were typed into it (Windows: Ctrl+Shift+P typed a "p" into the palette); `Form.Hide` did nothing on Wayland (winit has no hidden windows there: the window is dropped and made again); `PaneTitle` of a non-active MDI document renamed the active one. The differences left between OSes are the window systems': Wayland places windows itself (`Form.Center` / `Left` / `Top` are hints) and its tool windows keep their minimize / maximize buttons.

**Performance** (§6.2): desktop cold start to first frame 479 ms median (≤ 1.0 s); a 50-file project opened +177 ms (≤ 0.5 s); web cached start 131 ms (≤ 1.2 s), with a project 151 ms; web first visit at 50 Mbit/s 2.47–2.54 s with an uncompressed server (target 2.5 s; `tests/studio_perf_web.mjs`). The web runtime's wasm is 13.7 MB raw, 3.53 MB brotli (+0.7 MB brotli for rapidr-studio and the fonts' use).

**Plug-in points.**
- L-PANELS: the panes are named in `ide/panes.inc` `SetUpDock` (`projecttree`, `toolboxtree`, `propgrid`, `outlinetree`, `outputbox`, `problemsview`, `immediatebox`, `welcome`) — a public component replaces a stand-in by taking its pane name; `ShowProperties` is where the inspector is fed.
- L-EDVIEW: `CodeDoc(d)` (RCODEEDITOR) per document in `ide/documents.inc` (`OpenDocument`, `CodeChanged`, `SaveDocument`).
- L-DMODEL / L-SYNC: `DesignDoc(d)` (RDESIGNSURFACE) is still filled by the stand-in scanner `ScanForm`; the plug point comment there says how rapidr-designer's `Document` takes over once RDESIGNSURFACE has source members (`Source` ← `open_bytes` / `set_text`, an `OnSourceEdit` per `TextPatch` from `sync()`, Undo / Redo through the document). Not wired in this step: RDESIGNSURFACE has no source-text API yet, and the live sync policy is L-SYNC's.
- I3: RLANGUAGESERVICE (`Lang.Update`, `Outline`, `Diagnostics`) in `AnalyzeActive`.

**Next.**
- **Preview in classic** (approved by Robert): a View menu item, a tool bar toggle and a palette entry, one setting in Studio's settings, off by default; when on, Run and the designer show the program in the classic theme without touching its source. Waits for L-THEME's theme selection in development (a runtime override the session passes to the child — an environment variable or launch option — and the designer surface's theme).
- The designer and the property grid / toolbox stand-ins (L-DVIEW, L-PANELS); MDI windows' drop shadow; the web's top-level kernel frame in RapidR's look; the examples list read from the examples folder; `--interp` runners and native builds without the studio components; a Studio-only wasm (or a feature) to take the 0.7 MB back from programs' pages; web-ide/ and examples/ide.rr deleted once Studio covers them.

**Shared files touched** (minimal): `Cargo.toml`, `Cargo.lock`, `.gitignore`; rapidr-value `lib.rs`, `globals.rs` (ThemeColor), `theme.rs` (rapidr names), `ide_theme.rs` (new), `objects/{code,text,font,textedit,memo,menu,tree,mod}.rs`, `members.rs`, `mdi.rs` (`child_state`, `set_child_title`), `dock/{geometry,look,runtime}.rs`; rapidr-ui-kernel `components/{mod,coolbtn,mdi,dock,tree,image,statusbar,menubar}.rs`, `tooltip.rs` (new), `tree.rs`, `input.rs`, `focus.rs`, `tick.rs`, `paint.rs`, `lib.rs`, `dialogs.rs` (chrome font), `text/{mod,editor}.rs`; rapidr-ui-app `desktop.rs` (modal input), `file_dialog.rs`, `lists.rs`; rapidr-ui-host-winit `winit_host.rs` (modal focus, Wayland hide, synthetic keys), `menu.rs`, `dialogs.rs`; rapidr-ui-host-web `host.rs`; runtime-core `Cargo.toml`, `lib.rs`, `object.rs`, `studio.rs` (new), `ui/kernel.rs`, `ui/kernel/dialogs.rs`; runtime-web `Cargo.toml`, `lib.rs`, `object_web.rs`, `globals_web.rs`, `file_picker_web.rs`, `kernel_web.rs`, `studio_web.rs` (new); vm-host-native / vm-host-web; rapidr-session `process.rs` (`spawn_with_env`); rapidr-cli `Cargo.toml`, `main.rs`, `launch.rs`, `notices.rs`; registry `forms.toml`, `globals.toml`, `dialogs.toml`, `studio.toml` (new) and the generated docs / `web-ide/lang-data.js`; `docs/ide-components.md`; `tests/gui_parity_cases.mjs` (dock_manager's pane sizes under the 28 px header); `tools/real_input.py`; `tools/release/{prepare.sh,stage.py}`; `tools/third_party_notices.py`, `LICENSES.md`, `THIRD_PARTY_NOTICES.md`; icons (`design/icons/inventory.toml`, `tools/planned.py`, the generated set); `CHANGELOG.md`; this section. Not touched: ROADMAP.md, regress.sh.

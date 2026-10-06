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
| Data science | `rapidr-runtime-core/src/datascience.rs` (1,792: RNum on ndarray, RDataFrame on polars 0.46 lazy / csv / json, RPlot on plotters) and `rapidr-runtime-web/src/datascience_web.rs` (1,778: `Vec<f64>`, `Vec<Vec<String>>` frames, plots on an HTML canvas) | **Two implementations** (against rule 2), state in thread-local maps keyed by component name, no Parquet | **One implementation in I7** (engine decision D7) |
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
rapidr-frame       NEW? the one data-frame engine for every runtime (decision D7)
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

**One implementation** (rule 2): today RDataFrame is polars on the desktop and string columns on the web, RPlot plotters on the desktop and an HTML canvas on the web. I7 makes each one implementation for every runtime:
- **The frame engine** (decision **D7**): a first spike builds polars (MIT; `lazy`, `csv`, `json`, `parquet`) for `wasm32-unknown-unknown` and measures size, speed and build friction. If it fits (a separately loaded wasm module only when a program uses data frames, ≤ ~3 MB brotli), polars everywhere. If not, our own columnar engine (`rapidr-frame`: typed columns, the transforms above, CSV / JSON / Parquet readers with permissive crates) on every runtime, polars dropped. Either way, one engine, same results on all three runtimes, checked by the corpus-style comparison on data programs (`examples/demo_dataframe.rr`, `web_datascience.rr` …).
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
| Data frames | `polars` 0.46 (+ `parquet`) | MIT | Spike (D7); check Parquet codecs' crates (zstd, lz4, snappy, brotli — each permissive, verify) |
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

Release notes, per the project's messaging: full RapidQ compatibility on all three runtimes (native compiler, interpreter, web), extended (data science, data components, AI to come), an original implementation in pure Rust — not a clone of RapidQ or Delphi.

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
| **D7** | The one data-frame engine | polars everywhere (wasm spike); our own engine everywhere | Decide by the spike's numbers (size ≤ ~3 MB brotli as a lazily loaded module, build friction); polars if it fits |
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
5. **The frame engine on wasm** (D7): polars may be too large or not build. Mitigation: the spike first, our own engine as the planned alternative.
6. **A RapidR-written shell at scale** (D3): maintainability of a large BASIC program. Mitigation: the language service itself, conventions, moving heavy glue into components.
7. **AI safety and cost**: prompt injection, runaway tool loops, surprise bills. Mitigation: [docs/ide-ai.md](ide-ai.md)'s tiers, previews, caps, usage display.
8. **Scope**: ten stages. Mitigation: the release bar in §8, waves with clear owners, gaps closed before new work.

---

## Results

(Appended per stage as work lands, with dates, sizes, test counts and what changed in the plan.)

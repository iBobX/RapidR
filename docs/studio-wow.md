# RapidR Studio: the WOW, as a testable checklist

Robert's bar (2026-10-08): "the best IDE out there". That means better than Delphi and Xojo, a lot better than RapidQ's IDE and VB6, and better than Xcode too. The WOW is high priority.

This file turns that bar into items that can be checked. Each item has:
- an ID;
- what the best competitor does;
- what RapidR Studio does better;
- a priority;
- an acceptance test.

The plan's own documents still hold the design: [ide-plan.md](ide-plan.md) (stages, §6.2 budgets, §8 release bar), [ide-components.md](ide-components.md) (APIs) and [ide-ai.md](ide-ai.md). This file is the **release checklist** for the experience. The release-quality gate (7) in ROADMAP, "the WOW pass", means every P0 item here is green.

**Priorities.**
- **P0**: the first public release. Nothing ships until it passes.
- **P1**: the first updates after it (1.1, 1.2).
- **P2**: later.

**Starting point** (`development` @ `bb23d078`, 2026-10-08). Robert tried the preview and found:
- the designer drew boxes but couldn't resize them;
- the toolbox didn't add anything;
- the properties didn't work;
- there was no autocomplete;
- Tab typed a stray character.

§6, the gap table, explains why from the code. Lanes are fixing the designer, the panels / inspector, the code editor, the theme and the font. This file is what they steer by.

---

## 0. How items are accepted

**Scripted flows: `tests/studio_wow.mjs`** (new). It works like [tests/studio_flows.mjs](../tests/studio_flows.mjs):
- each case is `{ name, open, do, delay, dump }`;
- it runs on the desktop's headless host and on the web page;
- the dumps must match a regular expression on both hosts;
- captures are byte-identical between desktop and web at 1× and 2× where the case takes them.

`do` lists Studio's command ids (the `--do` path), `wait`, and these **test steps**, which the shell needs to accept. They go through the same input paths as a user: the kernel's key and mouse events, and `RAPIDR_TEST_EVENTS`'s `__mousedown` / `__mousemove` / `__mouseup` / `__key_N`.

| Step | Meaning |
|---|---|
| `type:<text>` | Typed one key at a time into the focused component. Completion, auto-case and auto-close all fire. |
| `key:<chord>` | One key press, e.g. `key:Tab`, `key:Ctrl+Space`, `key:Enter`, `key:F9`. `Ctrl` is ⌘ on macOS. |
| `focus:<pane>` | `Dock.FocusPane`. |
| `tool:<TYPE>` | A toolbox item picked and added the way Enter or a double click adds it. |
| `select:<Name>[,+<Name>]` | Components selected in the active designer by clicks, the second with Shift. |
| `drag:<Name>:<part>:<dx>,<dy>` | A mouse drag on the active designer. `part` is `body` or a handle: `n`, `ne`, `e`, `se`, `s`, `sw`, `w`, `nw`. |
| `drop:<TYPE>:<x>,<y>` | A toolbox item dragged and dropped on the designer at (x, y). |
| `prop:<Prop>=<value>` | The inspector's editor for that row, set as a user would set it. |
| `dblclick:<Name>` | A double click on a component in the designer. |
| `capture:<label>` | PNGs at 1× and 2× under `tests/results/studio-wow/`. |

Dump keys are the existing `component.property` keys, plus these:
- `designer.selected`;
- `designer.<Name>.bounds` (`L,T,W,H`);
- `code.text` (the active code document);
- `code.caret` (`line:col`);
- `completion.visible`, `completion.items` (the first 20 labels, comma-separated);
- `hover.text`;
- `inspector.rows`;
- `problems.count`.

**Measured budgets: `tools/regress.sh perf`** (new stage). Timings come from the hosts' frame clocks and `performance.now()`. `tests/studio_perf_web.mjs` is extended, and a desktop twin is added. A regression of more than 20 % fails, as [ide-plan.md](ide-plan.md) §6.4 says.

**Real input.** The first five minutes (§2) also run by hand or by script on:
- macOS, through `tools/real_input.py`;
- Windows 11 and Ubuntu (Wayland) in the VMs;
- Chromium, Firefox and Safari.

This is ROADMAP release gate (1).

---

## 1. Benchmark: what each IDE does best, and what users hate

The sources were checked on 2026-10-08: vendor documentation and release notes, G2 / Capterra reviews, the Xojo forum, Delphi-PRAXiS and the RapidQ docs on phatcode.net. Some claims couldn't be checked against a source; they are marked *(not sourced)* below.

| IDE | What users praise | What users hate |
|---|---|---|
| **Xcode 26** | SwiftUI previews: a canvas with variants (dark mode, Dynamic Type, orientation) and `#Preview` tabs. Interface Builder: Ctrl-drag to code for actions / outlets *(not sourced)*. The Library (⌘⇧L) and Open Quickly (⌘⇧O) *(not sourced)*. 26.3 exposes the IDE to agents (Claude Agent, Codex) **over MCP**: agents read the docs, edit, build and check previews. | Previews crash or freeze, especially in betas. Indexing is slow. Autocomplete often shows nothing until most of the line is typed. Opaque errors. Storyboard / pbxproj merge conflicts. Stepping stalls while symbols load. Signing pain. |
| **Delphi / RAD Studio 13** | The designer and the code stay in sync both ways. Object Inspector: non-default values bold, a search box (13), typed property editors, an Events tab where a double click makes the handler. Ctrl+arrows nudge. **IDE Insight** (Ctrl+. / F6): one search over commands, components on the form, options and files. Structure view. LiveBindings. | The language server (Code Insight) crashes, stalls, and stops until a restart; 13 brought the classic engine back by popular demand. Licence price. High-DPI issues. The Structure panel changes with focus. |
| **Xojo 2025** | One project builds macOS, Windows, Linux, web, iOS and Android. A remote debugger, including headless. Layout locks keep the arrow-key nudge. Cmd-double-click jumps to a declaration (r1). Wrap in Try…Catch (r2). Format standardized as you type (r3). Jade, an AI assistant (r3). | Autocomplete misses members of the user's classes and dies until a restart. The code editor uses a lot of CPU. The IDE is slow and buggy on big projects. Price, up a reported ~500 % over 20 years, and builds behind licences. Slow fixes. |
| **VB6** | F5 from design to run. Double-click a control to get its code. The Immediate window: re-run and edit lines, read anything. Edit-and-continue. Auto List Members and Quick Info. The Object / Event drop-downs above the code. | Dead since 2008. Forms saved as binaries. No 64-bit. DLL hell. Weak OOP. |
| **RapidQ IDE** (1999–2000) | Simple. F5 saves and runs. A form designer, and properties beside events. FreeQ later added Ctrl+. completion. | No debugger, by its own manual. Not every component is in the designer. Only "primitive" checks: RC.EXE finds the errors. No completion in the original. |
| **VS Code** | Completion with CamelCase / fuzzy filtering. F12, Alt+F12 (peek), Shift+F12, F2 rename across files. ⌘P quick open, ⌘⇧P palette. Multi-cursor, sticky scroll, minimap. Fast typing. | Language servers eat CPU and memory, and the fix is "restart the server". AI next-edit suggestions are intrusive. It's an editor, not a RAD tool: no designer. |

**The pattern.** Every competitor's top complaint is reliability: a language service that dies, previews that crash, autocomplete that misses the user's own code. The top praise goes to immediacy:
- design ↔ code in sync;
- double-click to a handler;
- F5 that just runs;
- Immediate / edit-and-continue.

RapidR's WOW is **all of the immediacy, none of the flakiness**. Some of that is already in place:
- one compiler-backed language model (`rapidr-langsvc` on bcgen's own semantic model, fuzzed on the corpus);
- one renderer, so the designed form is the running form, byte for byte;
- desktop and web from one source;
- every component described from the registry for people, screen readers and AI alike.

---

## 2. The first five minutes

A new user, on any OS or in a browser, goes from launch to a program built for three OSes and the web. Every step is P0, and every budget is measured (`tools/regress.sh perf`). The whole flow is case `first-five-minutes` in `tests/studio_wow.mjs`, and it also runs with real input (§0).

| # | The user does | Studio does | What makes it delightful | Budget |
|---|---|---|---|---|
| 1 | Launches RapidR Studio: double-click, `rapidr ide`, or opens the web page. | Shows the Welcome page in the theme the OS uses. Focus is on **New Project**. | No account, licence key, SDK download, setup wizard, telemetry prompt or AI nag. Xcode, Delphi and Xojo all ask for at least one of these. | Cold start ≤ 1.0 s desktop (measured: 479 ms); web ≤ 2.5 s first visit, ≤ 1.2 s cached. |
| 2 | Looks at Welcome. | Shows Start (New, Open, Example, All Commands), Recent, and Examples as cards. Each card shows a **live thumbnail of its form** and a **▶ Run** button. | Any example runs in one click without opening anything. Keyboard: arrows move between cards, Enter opens, Shift+Enter runs. | Thumbnails painted ≤ 100 ms after the page shows. |
| 3 | Presses ⌘N / Ctrl+N. | The New Project gallery appears: template cards (§3.1 WEL-2), each with a live thumbnail. The name is pre-selected, the folder defaults to `~/RapidR Projects`, and a "RapidQ-compatible" checkbox is offered. | Type a name and press Enter: done. Nothing else to configure. | Dialog ≤ 100 ms; project created and opened ≤ 500 ms. |
| 4 | Lands in the project. | The form's designer and its code sit side by side (MDI tiled). The inspector shows Form1, the toolbox is open, Outline and Problems are filled. | It looks finished: Inter / JetBrains Mono, the D8 icons, no empty grey panes (§4.3 empty states). | First paint ≤ 500 ms (measured: +177 ms for a 50-file project). |
| 5 | In the toolbox, types `but` and presses Enter. | A QBUTTON (Button1) appears at the next free spot of the selected container, already selected. The inspector's active row is **Caption**. | Typing `Greet` goes straight into Caption, as in Delphi; the button updates as the user types. The code pane shows the new `CREATE Button1 AS QBUTTON … END CREATE`, with the changed lines flashing once. | Add → visible ≤ 1 frame (16 ms); keystroke → designer + code ≤ 1 frame. |
| 6 | Adds a QEDIT by dragging it from the toolbox. | Guides snap the edit to the button's left edge, baseline and the 8 px margin, with distance labels. Dragging the edit's right handle shows a live `W × H` readout. Clicking the right **anchor pin** and then dragging the form's corner shows the edit stretching. | This is Xcode / Xojo guides, plus anchoring that matches the running program exactly (a test compares them). Alt suspends snapping. | 60 fps while dragging; guides ≤ 2 ms per move (model measured < 2 ms with 300 components). |
| 7 | Double-clicks Button1. | `OnClick = Button1Click` is added to the CREATE block. `SUB Button1Click (Sender AS QBUTTON) … END SUB` is created with the registry's parameters. The editor scrolls there with the caret inside. | VB6 / Delphi muscle memory, plus the minimal text edit: user code stays byte-identical. | ≤ 1 frame. |
| 8 | Types `showmessage "Hello, " + Edit1.` | Auto-case turns `showmessage` into `SHOWMESSAGE` on the space. Signature help shows `SHOWMESSAGE(Text)`. After `Edit1.`, completion lists QEDIT's members with icons and docs. `Te` + Tab gives `Text`. | It knows the program: locals, members by type, WITH, CREATE bodies, Q and R names. The service comes from the compiler, so it can't disagree with the build. | Completion p95 ≤ 50 ms desktop, ≤ 100 ms web. |
| 9 | Makes a typo (`Edit2.Text`). | A squiggle appears in RapidQ's own wording within 300 ms. ⌘. offers "Change to Edit1". | The diagnostics are exactly the compiler's (same message, same column). | ≤ 300 ms after typing stops. |
| 10 | Presses F5 (or ⌘R on macOS). | Saves, then runs. The form appears; clicking Greet shows the message. The status bar turns green; Output shows the program's PRINTs. | VB6's F5, with nothing to configure. The program runs in its own process, so a crash can't take Studio down. | F5 → first form ≤ 300 ms desktop, ≤ 500 ms web. |
| 11 | Presses F9 on the SHOWMESSAGE line, F5, then clicks Greet. | Stops on the line, highlighted in the editor. Locals show `Sender`. Hovering `Edit1.Text` shows its value. In Immediate, `? Edit1.Text` prints it. F5 continues. | VB6's debugger, without its age. Pausing and inspecting work the same on the desktop and the web. | Stop → highlight and locals ≤ 100 ms. |
| 12 | Picks **Run ▸ Run in Browser**. | The default browser opens a local page (loopback, random port) running the same program. | One source on the desktop and the web, pixel-identical. Nobody else does this from a RAD designer. | ≤ 1.5 s to the form in the browser. |
| 13 | Picks **Build ▸ Build for…**, keeps macOS, Windows, Linux and Web ticked, and clicks Build. | Progress per target. Then the output folder opens: a macOS executable (universal), Windows `.exe`s (x64 + ARM64), Linux executables (x86_64 + aarch64) and a `web/` folder. Each has its `THIRD-PARTY-NOTICES.txt`. | Every OS from any OS in one click, with no paid licence tier (Xojo / Delphi gate this). Interpreted standalone executables come from the shipped runners (`rapidr build --interp --target`). Native "Release for this machine" is one more checkbox. | All four interpreted targets ≤ 10 s; web ≤ 3 s. |
| 14 | Quits, then opens Studio again. | Recent shows the project. Opening it restores the layout, open documents and breakpoints. | Picks up where the user left off. | — |

**Test.** `first-five-minutes`:
- open nothing;
- do `file.newProject,type:Hello,key:Enter,wait,focus:toolbox,type:but,key:Enter,type:Greet,tool:QEDIT,dblclick:Button1,type:SHOWMESSAGE Edit1.Te,key:Tab,run.start,wait,wait,run.stop,run.build.all,wait`;
- dump:
  - `code.text` must match `/CREATE Button1 AS QBUTTON[\s\S]*Caption = "Greet"[\s\S]*OnClick = Button1Click[\s\S]*SUB Button1Click \(Sender AS QBUTTON\)[\s\S]*SHOWMESSAGE Edit1\.Text/`;
  - `problems.count = 0`;
  - `session.exitcode = 0`;
  - the build folder lists the four targets.

On the web, the build step checks the web bundle and lists the OS targets as "desktop Studio".

---

## 3. Per area

Each table's columns are: ID | item | the best competitor | RapidR better, concretely | P | acceptance.

### 3.1 Welcome and templates (WEL)

| ID | Item | Best competitor | RapidR better | P | Acceptance |
|---|---|---|---|---|---|
| WEL-1 | Welcome page | Xcode's welcome window: recent projects plus "Create" | Start, Recent and Examples on one page that reflows. Example cards have live form thumbnails and ▶ Run. Fully keyboard-driven. | P0 | `welcome-run`: `welcome.__key_Down…`, Shift+Enter on the "hello_form" card → the program runs (`session.state=running`) without a project being opened. Capture compared desktop / web. |
| WEL-2 | Template gallery | Xojo / Xcode template choosers; Delphi's Object Gallery | At least 5 templates, each with a kernel-drawn thumbnail of its real form, and each runs on the desktop and the web: **Form app**, **Console**, **RapidQ-compatible form app** (compat on, Q names only), **Data dashboard** (CSV → grid + chart), **MDI app with menus**. | P0 | `templates`: for each template, create it → `run.start` → exit 0 on both hosts. `rapidr-project` `TEMPLATES` lists all 5. |
| WEL-3 | Zero setup | All three ask for an account, a licence key or SDK downloads | Nothing asked on first run. The theme follows the OS. No network access on first run (checked). | P0 | Fresh profile (`--fresh`): no dialog in the first 5 s (`Screen.FormCount`); a socket trace shows no connections. |
| WEL-4 | Open by dropping | VS Code: drop a folder or file | Dropping a `.bas` / `.rr` / `.rrproj` / folder on the window (desktop) or the page (web) opens it. | P1 | Real-input check per OS. |
| WEL-5 | Import RapidQ / VB6 | — | The RapidQ folder importer (ROADMAP Phase 1) at P1; the VB6 `.vbp` importer at P2. | P1/P2 | Corpus folder → project → compatibility report. |
| WEL-6 | Tips strip | Xojo / Delphi "tip of the day" (disliked when modal) | One line, never modal, dismissible, keyboard-reachable. | P2 | — |

### 3.2 Designer (DES)

| ID | Item | Best competitor | RapidR better | P | Acceptance |
|---|---|---|---|---|---|
| DES-1 | WYSIWYG | Xojo / Delphi draw real controls; Xcode previews (which crash) | The real kernel components, drawn in the program's own `$THEME` and scale. The designed form equals the running form **byte for byte** on desktop and web. | P0 | `wysiwyg`: open `examples/gui/pantry.rr`, capture the designer's form area and the running form; the pixel diff is 0 at 1× and 2×, desktop and web. |
| DES-2 | Select, move, resize | Xcode / Xojo | Click, Shift / ⌘-click, rubber band, 8 handles, a live `W × H` / `X, Y` readout, arrows nudge 1 px, Shift+arrows nudge by the grid, ⌘ / Ctrl+arrows resize. Alt suspends snapping. Every move is persisted to code. | P0 | `resize`: open `hello_form.rr`, `select:GreetButton,drag:GreetButton:e:+30,0`; `designer.GreetButton.bounds=/112,52,120,\d+/`, `code.text` has `Width = 120` and no other line changed (diff of one line). |
| DES-3 | Smart guides | Xcode / Xojo alignment guides | Edges, centres, **baselines**, parent centre, 8 px margins, and equal spacing with distance labels. Snap threshold 5 logical px at any zoom. | P0 | Unit tests (`rapidr_value::designer::snap`, done). `guides` capture case while dragging. 300 components ≤ 2 ms per move (perf). |
| DES-4 | Add components | Delphi: click the palette, then click or draw; Xcode: drag from the Library | Double-click or Enter adds at a free spot in the selected container. Dragging drops with guides. Click a tool, then **draw** the rectangle (Delphi). Names are unique (Button1, Button2). Typing right after adding edits Caption (Delphi). | P0 | `toolbox-add`: `tool:QBUTTON,type:OK` → `code.text` has `CREATE Button1 AS QBUTTON` with `Caption = "OK"`; `drop:QEDIT:40,80` → `designer.Edit1.bounds=/40,80,/`. |
| DES-5 | Two-way sync | Delphi (design ↔ source), with `.dfm` files | **The source is the form**: no designer files, no marker comments. Each designer action is the smallest text edit, on one undo history with the code. A code edit updates the designer ≤ 150 ms after typing pauses. While the code doesn't parse, the designer keeps the last good state behind a "code has errors at line N" banner. | P0 | `sync`: designer move → one-line change; `type:` into a CREATE block's `Left = 200` → `designer.X.bounds` follows; a syntax error → banner visible, designer read-only; undo restores exact bytes (hash). Corpus round trip (done in `rapidr-designer`). |
| DES-6 | Double-click → handler | VB6 / Delphi | Creates the default event's SUB with the registry's parameters and wires `OnX = Handler`, or jumps to an existing one. The caret lands inside. | P0 | `handler`: `dblclick:GreetButton` on `hello_form.rr` → `code.caret` is inside `SUB Greet`; on a new button → a new SUB plus the `OnClick =` line. |
| DES-7 | Anchors and the resize preview | Xcode's Auto Layout (complex); Delphi anchors in the inspector only | Pins on the four sides toggled on the canvas or by keyboard. Dragging the form's corner shows a preview that **equals the runtime** (the user's must, 2026-10-06). | P0 | `designer_anchors` (done in the model: 40 / 40 values) re-run through Studio: `drag:Form:se:+120,+80` → bounds equal the gui_parity expectations. |
| DES-8 | Align, distribute, same size, z-order | Delphi's Align palette; the web IDE's commands | The Format menu works on multi-selection, aligned to the first selected or to the parent. Each command is one undo step. | P0 | `align`: `select:A,+B,designer.alignLeft` → equal Left values; a single undo restores them. |
| DES-9 | Containers | Delphi / Xojo | Dropping into a panel, group box, tab page or scroll box reparents (the CREATE nesting moves) with a target highlight. | P0 | `reparent`: `drag:Button1:body` into `Panel1` → `code.text` has Button1's CREATE inside Panel1's. |
| DES-10 | Undo / redo | Xcode Interface Builder crashes on complex undo | ⌘Z / ⌘⇧Z undo every designer and code change, in one history per file, restoring exact bytes. | P0 | Property test (done in `rapidr-designer`); Studio case: 10 random actions, undo × 10 → original hash. |
| DES-11 | Cut, copy, paste, duplicate | Delphi (copies as text) | ⌘D duplicates offset by the grid. Paste keeps names unique and handlers unbound. The clipboard holds CREATE text, so pasting into the code editor gives the source. | P0 | `duplicate`: `select:GreetButton,key:Ctrl+D` → `GreetButton2`, no second `OnClick = Greet`. |
| DES-12 | Keyboard-only design | Nobody does this well (Xcode IB is weak with VoiceOver) | Tab / Shift+Tab move through components, Esc goes to the parent, Enter in the toolbox adds, arrows move. Every change is announced: "Button1, QBUTTON, 112, 52, 90 by 25". | P0 | a11y dump (`web_a11y`-style): announcements present; a keyboard-only `first-five-minutes` variant passes. |
| DES-13 | Menu editor and Tab-order editor | VB6's Menu Editor; Delphi's | RMenuEditor inline on the form's menu bar (captions, `&`, shortcuts, checked, separators, OnClick). RTabOrderEditor as a list with Up / Down, by RapidR's rule. | P0 | `menus`: build File ▸ Open in the editor → the QMAINMENU CREATE text equals the expected; tab order list → `TabOrder =` lines. |
| DES-14 | Component tray | Delphi's non-visual components on the form; Xcode's scene dock | Timers, dialogs and data components in a tray under the form, with links shown as lines on demand. | P0 | `tray`: `tool:QTIMER` → in the tray, not on the form; the code gets a CREATE. |
| DES-15 | Live data at design time | Delphi LiveBindings (design-time data) | A CSV dropped on the form becomes an `RDataFile`; an `RPlot` pointed at it draws **live in the designer** and redraws when the file changes. Read-only at design time. ([ide-plan.md](ide-plan.md) I7 core.) | P0 | I7's acceptance: 100,000-row CSV → plot in the designer ≤ 300 ms; editing the file updates it. |
| DES-16 | Preview in classic | — | Toggling "Preview in classic" makes the designer and Run show RapidQ's exact Windows look without touching the source (approved). | P0 | `classic-preview`: toggle → designer capture equals the classic-theme run capture. |
| DES-17 | Inline caption edit | Xojo / Xcode (double-click a label) | F2 or Enter edits Caption / Text in place on the canvas. A double click stays "go to handler" (VB6 rule). | P1 | — |
| DES-18 | Preview variants | Xcode previews: dark, Dynamic Type, devices | Preview in light / dark / high contrast / classic and at 1×, 1.5× and 2×, side by side. | P1 | — |
| DES-19 | Drag to code (connections) | Xcode: Ctrl-drag to code for IBAction | Dragging a component onto a SUB in the code wires it as that event's handler (compatible signatures only). | P2 | — |
| DES-20 | Phone / tablet preview | Xojo iOS / Android layouts | With mobile (ROADMAP Phase 7). | P2 | — |

### 3.3 Inspector (INS)

| ID | Item | Best competitor | RapidR better | P | Acceptance |
|---|---|---|---|---|---|
| INS-1 | Rows from the registry | Delphi's Object Inspector | Every design-time property of the selection, by category or A–Z. Non-default values in **bold**. A search box. RapidR extensions badged "R". Rows set in code outside CREATE are greyed with "set in code, line N". | P0 | `inspector`: `select:GreetButton` → `inspector.rows` includes `Caption`, `Default`, `OnClick`; Default is bold (dump `inspector.bold`). |
| INS-2 | Typed editors | Delphi's property editors | Number, text, Boolean (checkbox), enum (drop-down of RapidQ constant names), set, colour (picker with RapidQ BGR and system colours `clBtnFace`…), font, picture / asset, strings list, columns, component reference picker. An edit reaches the designer **and** the code ≤ 1 frame. | P0 | `prop-edit`: `prop:Caption=Go` → designer caption and `code.text` `Caption = "Go"`; `prop:Color=clRed` writes the constant; `prop:Default=False` writes `Default = 0` or removes the line (RapidQ spelling). |
| INS-3 | Events tab | Delphi; Xojo's "Add Event Handler" | Every event with its parameters. A double click creates or jumps to the handler; the drop-down lists existing compatible SUBs. | P0 | `events-tab`: double-click OnChange → SUB created; pick an existing SUB → `OnChange = Name` only. |
| INS-4 | Multi-selection | Delphi | Common properties; mixed values shown blank; one edit sets all of them in one undo step. | P0 | `multi`: `select:A,+B,prop:Width=100` → both 100, one undo. |
| INS-5 | Inline docs | Delphi's help pane; Xcode's Quick Help | The focused row's registry doc (our own words), type, RapidQ default and origin, under the grid. F1 opens the full entry. | P0 | `inspector-doc`: focus the Caption row → `inspector.doc` non-empty. |
| INS-6 | Reset to default | Delphi (right-click → Revert) | Delete or a right-click resets the property and removes its line from the CREATE block. | P0 | Line removed; undo restores it. |
| INS-7 | Scrub numbers | Xcode / Figma: drag on the label | Dragging a number row's label changes the value live. | P1 | — |
| INS-8 | Live into the running program | Xojo: none; Xcode: previews only | While the program runs, an inspector edit is also applied to the running form (I5). | P1 | I5 acceptance: ≤ 100 ms. |

### 3.4 Toolbox / Library (TBX)

| ID | Item | Best competitor | RapidR better | P | Acceptance |
|---|---|---|---|---|---|
| TBX-1 | From the registry | Delphi's palette; Xojo's Library | Every component the registry knows (not a hand list), grouped "RapidQ" (Standard, Additional, Dialogs, System, Media) and "RapidR" (Data, Data Science, Web, AI, IDE), with D8 icons and names as written (QBUTTON, RPLOT). | P0 | `toolbox-registry`: the item count equals `rapidr lang export --json`'s visual + tray components (minus `planned`). |
| TBX-2 | Search | Xcode's Library (⌘⇧L) filter; Delphi 13 | Type to filter by name, Q / R name and doc words ("chart" finds RPLOT). Enter adds the first match. | P0 | `toolbox-search`: `focus:toolbox,type:chart` → first item RPLOT. |
| TBX-3 | Add | — | See DES-4: double-click, Enter, drag, click then draw. Non-visual components go to the tray. | P0 | DES-4 cases. |
| TBX-4 | Hover card | Xcode's Library detail pane | Icon, one-line doc, origin, "desktop only" / "web only" flags, a "Help" link. | P0 | `hover.text` on an item is non-empty and includes the origin. |
| TBX-5 | Floating library | Xcode's ⌘⇧L at the mouse | ⌘⇧L opens a searchable library at the selection; Enter adds. | P1 | — |
| TBX-6 | Templates | Delphi's component templates | Save a selection as a template (preset properties and handlers' text), shown under "My templates". | P1 | — |

### 3.5 Code editor and IntelliSense (ED)

| ID | Item | Best competitor | RapidR better | P | Acceptance |
|---|---|---|---|---|---|
| ED-1 | A real editor | VS Code | `rapidr-editor` (done as a model) wired into RCODEEDITOR: undo / redo grouped by word, multi-cursor (⌘D, ⌥-click), find / replace with regex (⌘F, ⌘H), bracket matching, auto-indent, folding, the current line, colours per theme. | P0 | `editor-basics`: `type:`, then `edit.undo` restores; `edit.find` with a regex selects the match; a fold hides `SUB … END SUB`. Model property tests (done). |
| ED-2 | Tab and indentation | — | Tab / Shift+Tab indent and outdent by the file's unit (keeps tabs if the file uses tabs, otherwise 4 spaces). Tabs draw as whitespace to the tab stop, never as a glyph. Tab also accepts completion and moves through snippet stops. | P0 | `tab`: `key:Tab` at a line start → 4 spaces (or `\t` in a tab file); capture shows no glyph; `key:Shift+Tab` removes it. |
| ED-3 | Completion | VS Code (fuzzy, CamelCase); Xcode and Xojo miss user code | From `rapidr-langsvc` (the compiler's model): locals, globals, SUBs, TYPEs, components, members by type (WITH, CREATE bodies, EXTENDS, `Item(i).`), Q and R names, snippets. Opens after `.`, after `AS `, while typing identifiers, and on Ctrl+Space. Fuzzy filter; Tab / Enter accept; the doc shows beside the list. **It never dies**: requests are answered from the last good model. | P0 | `completion`: `type:NameEdit.` → `completion.items` includes `Text,SelStart`; `type:Te,key:Tab` → `NameEdit.Text`; p95 ≤ 50 ms (perf). Golden suite `tests/langsvc/` (done). |
| ED-4 | Hover and signature help | Xcode Quick Help | Hover: declaration, type, Q / R names, registry doc, flags. Signature help with the active parameter, including SUB calls without parentheses. | P0 | `hover`: `hover.text` on `SHOWMESSAGE` includes its syntax. |
| ED-5 | Live diagnostics and quick fixes | — (Delphi's Error Insight is unreliable) | Squiggles in RapidQ's wording, identical to `rapidr build-bc`, ≤ 300 ms after typing stops. ⌘. shows quick fixes (Did you mean…, QPLOT → RPLOT, declare it). Problems stays in sync. | P0 | `diagnostics`: `type:Edit2.Text` → a squiggle at the right column; `problems.count=1`; quick fix → 0. |
| ED-6 | Navigation and rename | VS Code (F12, Shift+F12, F2) | F12 / ⌘-click go to definition, including `$INCLUDE` lines. Shift+F12 finds references. F2 renames across files from the semantic model and refuses on a clash. ⌃- goes back. | P0 | `navigate`: F12 on `Greet` → the SUB line; F2 `Greet→SayHi` → every reference and `OnClick = SayHi`. |
| ED-7 | Automatic keyword case | QuickBASIC / VB | As you type (`langsvc::case`, done in VS Code), set per project: upper, lower, proper or preserve. | P0 | `case`: `type:dim x as integer ` → `DIM x AS INTEGER `. |
| ED-8 | Snippets | VS Code | `sub`, `function`, `create`, `for`, `select`, `type`, with tab stops, from the language definition. | P0 | `snippet`: `type:sub,key:Tab` → a `SUB … END SUB` skeleton with the caret at the name. |
| ED-9 | IME and accessibility | VS Code on the web (textarea window) | IME on every host; AccessKit text runs; line / column and diagnostics announced. | P0 | [ide-plan.md](ide-plan.md) I2 acceptance (screen readers by hand, IME CJK). |
| ED-10 | Object / Event bar | VB6's two drop-downs | Above the code: a component on the left; on the right its events (bold where handled). Picking one creates or jumps to the handler. | P1 | — |
| ED-11 | Formatter | VS Code's Format Document | ⇧⌥F re-indents and sets case; it never changes a token (exists in `rapidr-langsvc`). | P1 | — |
| ED-12 | Sticky scroll, minimap, inlay parameter names, semantic colours, bookmarks, compare with saved (RDiffView) | VS Code | — | P1 | — |
| ED-13 | AI inline completion | Copilot (users call it intrusive) | Off by default; on only when AI is configured; never while the completion list is open. | P2 | — |

### 3.6 Project tree (PRJ)

| ID | Item | Best competitor | RapidR better | P | Acceptance |
|---|---|---|---|---|---|
| PRJ-1 | Files and forms | Delphi's Project Manager; Xojo's Navigator | Files by kind, with D8 icons and the main file marked. Forms expand to their components; selecting one selects it in the designer. Dirty files are marked. | P0 | `tree-components`: expanding hello_form shows 4 components; selecting one gives `designer.selected`. |
| PRJ-2 | New, rename, delete | All | New Form / Module / File (`project.addForm` / `addModule` work). Rename (F2) updates `$INCLUDE`s. Delete asks and goes to the trash. | P0 | `tree-edit`: `project.addForm` → `Form2.rr` in the tree and in `.rrproj`; rename → the include line follows. |
| PRJ-3 | Search the tree | Delphi 13 (incremental, 20,000+ files) | Type to filter. | P1 | — |
| PRJ-4 | Watch the disk; git badges | VS Code | — | P1 / P2 | — |
| PRJ-5 | Assets | Xojo's project items; the old web IDE's assets manager | The project's files (pictures, sounds, data) with previews (image, text, CSV), filter, rename that updates every reference, delete; INS-2's picture editor picks from them; carried by `.rrproj` and every build. | P0 (proposed) | `assets`: add a PNG → listed with its preview; rename → the `Picture =` line follows; `run.build.web` → the bundle has it. |

### 3.7 Run (RUN)

| ID | Item | Best competitor | RapidR better | P | Acceptance |
|---|---|---|---|---|---|
| RUN-1 | F5 runs | VB6 | Save and run (RapidQ saved on run too) in its own process or sandboxed frame. Output shows ANSI (COLOR, LOCATE). On macOS ⌘R runs too. | P0 | `run-console` (done) + `run-ansi`: COLOR output → coloured runs in `outputbox`. |
| RUN-2 | Run in browser | — | From desktop Studio, one command serves the web build on loopback (random port, token) and opens the default browser. | P0 | `run-browser`: the page's first form appears ≤ 1.5 s; its capture equals the desktop run's. |
| RUN-3 | Errors before running | Xojo / Delphi stop at the first error | F5 with errors: Problems is shown and the editor **jumps to the first error**, with no dead run. | P0 | `run-errors`: a broken file + `run.start` → `code.caret` on the error's line. |
| RUN-4 | Run-time errors | VB6 | Stops at the faulting statement (break on error, done in the VM), with the message and the call stack. The line is highlighted. | P0 | `run-error-stop`: a division by zero in a handler → paused at that line, the `Stopped` reason shown. |
| RUN-5 | Stop and restart | — | Stop always works (a kill); Restart is ⌃⇧F5. | P0 | A busy-loop program + `run.stop` → stopped ≤ 200 ms. |
| RUN-6 | Remote run / debug | Xojo's remote debugger | Run and debug on another machine or VM: the session protocol over an authenticated TCP link. | P2 | — |

### 3.8 Debug (DBG)

| ID | Item | Best competitor | RapidR better | P | Acceptance |
|---|---|---|---|---|---|
| DBG-1 | Breakpoints | VB6 / Delphi | F9 or a gutter click, in any file. A condition is set by right-click and edited inline. Breakpoints are kept in `.rapidr/workspace.toml`. | P0 | `breakpoint`: F9 on `Greet`'s line, run, click → paused at it (`session.currentline`); a condition `clicks > 1` stops only on the second click. |
| DBG-2 | Stepping | — (Xcode stalls while loading symbols) | F10 / F11 / ⇧F11, Run to Cursor (⌃F10), Pause. The current line is highlighted and the editor follows across files. | P0 | `step`: F11 into a SUB in an `$INCLUDE` → that file opens at the line. |
| DBG-3 | Variables and watches | Delphi's Local Variables / Watches | A Locals / Globals tree; objects expand to their properties; arrays page by 100. A Watch panel evaluated by the VM (fuel-limited). | P0 | `variables`: paused → `variables.locals` includes `Sender`; a watch `clicks*2` updates on the next stop. |
| DBG-4 | Call stack | All | A panel; clicking a frame shows its locals and line. | P0 | `callstack`: two frames, the second's locals differ. |
| DBG-5 | Data tips | Delphi / VS Code | Hovering a variable while paused shows its value; objects expand. | P0 | `hover.text` while paused is `= "World"`. |
| DBG-6 | Immediate | VB6 | `? expr` prints; statements run in the paused frame. Up / Down for history, completion in the line. | P0 | `immediate` (works now when paused: `ide/project.inc` `ImmediateKey`): `? NameEdit.Text` → `World`. |
| DBG-7 | Logpoints, hit counts, set next statement | VS Code / Delphi | — | P1 | — |
| DBG-8 | Edit-and-continue and hot reload | VB6 (still loved) | I5: a changed SUB applies on save while running; inside a paused SUB if its locals' layout is kept; otherwise "restart needed" with the reason. | P1 | I5 acceptance. |
| DBG-9 | Inspect element | Xcode's view debugger | Click a control in the running form → it's selected in the designer with its live properties (I5). | P1 | — |
| DBG-10 | Event timeline, reverse stepping, line profiler | — | ROADMAP Phase 2. | P2 | — |

### 3.9 Build and package (BLD)

| ID | Item | Best competitor | RapidR better | P | Acceptance |
|---|---|---|---|---|---|
| BLD-1 | Build for every OS | Xojo: one project to every OS, but behind paid licences; Delphi's Linux needs Enterprise | **Build ▸ Build for…** (command `run.build.all`): tick macOS, Windows, Linux, Web, then one click. Interpreted standalone executables for every OS and architecture **from any OS** (shipped runners: `rapidr build --interp --target <os>-<arch>`). Progress per target; the output folder opens. | P0 | `build-all`: `run.build.all` on `hello_form` → `MyProgram` (macOS universal), `.exe` x64 + ARM64, Linux x86_64 + aarch64, `web/index.html`; each runs (VMs for Windows / Linux) and carries `THIRD-PARTY-NOTICES.txt`. |
| BLD-2 | Native build | Delphi / Xcode | "Release, native, this machine" through the shipped toolchain. Compile errors go to Problems; the cargo log goes to Output. | P0 | `build-native`: the native `hello_form` runs and its window capture equals the interpreted one. |
| BLD-3 | Web bundle | Xojo Web (needs a server) | A static folder or zip (no server needed) with notices, and a "Preview bundle" command. | P0 | `build-web`: the bundle served statically shows the form. |
| BLD-4 | App identity | Xcode's target settings | The name, icon (`$OPTION ICON`), version and theme come from Project Options and are applied to every target. | P0 | The icon is visible in the macOS app and in the `.exe`'s resources. |
| BLD-5 | Installers | Xojo / Delphi (third-party installers) | macOS `.app` + `.dmg`, a Windows installer, Linux AppImage / `.deb`, from Studio (RapidR's own `tools/release/` know-how). Signing settings use the user's certificates. | P1 | — |
| BLD-6 | Web deploy | Xojo Cloud (paid) | Deploy to any static host (a folder, GitHub Pages) from Studio. | P1 | — |
| BLD-7 | Native cross-builds | — | Native executables for other OSes. | P2 | — |

### 3.10 Search and command palette (CMD)

| ID | Item | Best competitor | RapidR better | P | Acceptance |
|---|---|---|---|---|---|
| CMD-1 | Command palette | VS Code (⌘⇧P) | Every command with its shortcut and icon; fuzzy; recent first; `:N` goes to line N (done). | P0 | `palette` (done) + `palette-fuzzy`: `type:tglbrk` → Toggle Breakpoint first. |
| CMD-2 | One search for everything | Delphi's IDE Insight; Xcode's Open Quickly | ⌘P searches files, symbols (`@`), components on the open form, properties, commands (`>`), settings, docs (`?`) and examples, in one box. Enter acts: open, select, go to, run, or show the doc. | P0 | `quick-open`: `type:@Greet` → `code.caret` at `SUB Greet`; `type:GreetBu` → the component selected in the designer. |
| CMD-3 | Find in Files | VS Code | ⌘⇧F with a results pane, replace across files (one undo per file), regex. | P0 | `find-in-files`: finds 2 hits across a project with an include. |
| CMD-4 | Settings page | VS Code | Searchable; no JSON. | P1 | — |

### 3.11 Help (HLP)

| ID | Item | Best competitor | RapidR better | P | Acceptance |
|---|---|---|---|---|---|
| HLP-1 | F1 in place | VB6's MSDN, RapidQ's `.chm`, Xcode Quick Help | F1 on any word in the editor, inspector row or toolbox item opens its **registry entry** in a docked Help pane: syntax, parameters, types, RapidQ default, Q / R origin, runtimes, an example, and an Insert button. Offline, the same on the web. | P0 | `help`: F1 on `SHOWMESSAGE` → `help.title=SHOWMESSAGE`, the syntax shown. |
| HLP-2 | Docs on hover everywhere | — | Completion, the inspector, the toolbox and hovers all show the same doc text (one registry). | P0 | Covered by ED-4, INS-5, TBX-4. |
| HLP-3 | Examples by topic | Xojo's examples folder | Examples searchable by the components they use ("examples with QLISTVIEW"). | P1 | — |
| HLP-4 | Explain this error | — | Each diagnostic links to a page with the cause and fixes. | P1 | — |

### 3.12 AI assistant (AI)

The release plan ([ide-plan.md](ide-plan.md) §8, D14) keeps I8 out of the first release, with a read-only MCP server as a stretch goal. ROADMAP release gate (7), however, names "AI help" in the demo. **Proposal for Robert:** make the read-only MCP server P0 (low risk, high WOW: Claude Code reads a RapidR project correctly from day one), and keep the assistant at P1.

| ID | Item | Best competitor | RapidR better | P | Acceptance |
|---|---|---|---|---|---|
| AI-1 | Read-only MCP server | Xcode 26.3 exposes the IDE to Claude Agent / Codex over MCP | `rapidr mcp` (a user-only socket and a token). Tools: project, files, outline, diagnostics, registry docs, form trees (components and properties), data schema. Every component is described from the registry. | P0 (proposed) | An MCP client test harness lists the tools and reads a form tree; a connection without the token is refused. |
| AI-2 | Assistant panel | Xcode 26.3, Xojo's Jade | Multi-provider, **local models first**. Nothing is sent until opt-in. Context chips. Every edit shown in RDiffView before it applies. One-step undo of a turn. "What was sent" for every request. | P1 | [ide-ai.md](ide-ai.md) acceptance. |
| AI-3 | Mutating tools | Xcode 26.3's agents | Add components, set properties, write handlers, run, read output, screenshot the running form — each tool a component's tool provider, behind permission tiers. | P1 | — |
| AI-4 | Fix and explain | — | "Fix with AI" on a diagnostic; "Explain this crash" on a run-time error. | P1 | — |
| AI-5 | Screenshot → form; AI driving the running app (RAI) | — | — | P2 | — |

### 3.13 Themes (THM)

| ID | Item | Best competitor | RapidR better | P | Acceptance |
|---|---|---|---|---|---|
| THM-1 | RapidR light / dark / high contrast | Xcode follows the system | Follows the OS **live** (a change while Studio runs), on desktop and web. WCAG AA everywhere, AAA in high contrast (tested). | P0 | `theme-live`: a simulated OS change → `application.theme` follows without a restart; contrast test (done for icons; extend to chrome). |
| THM-2 | Editor colours per theme | VS Code | Token colours from the theme table; readable in dark (done: `ide_theme::editor`). | P0 | Contrast test of every token kind against its background. |
| THM-3 | The program's theme ≠ Studio's | — | The designer and Run show the program in its own `$THEME`, independent of Studio's. Preview in classic (DES-16). | P0 | Studio dark + program default → the designer capture equals the program's RapidR-light run. |
| THM-4 | Accent colour, imported colour schemes | VS Code | — | P1 | — |

### 3.14 Accessibility (A11Y)

| ID | Item | Best competitor | RapidR better | P | Acceptance |
|---|---|---|---|---|---|
| A11Y-1 | Keyboard everywhere | Visual Studio | F6 between areas (done), Ctrl+Tab between documents (done), every command in the palette or on a shortcut, designing by keyboard (DES-12), visible focus rings. | P0 | A keyboard-only `first-five-minutes` passes. |
| A11Y-2 | Screen readers | Xcode IB, Delphi and Xojo designers are mouse-centric | VoiceOver, NVDA and Orca reach every pane, toolbar button, tree node, inspector row, completion item and designer component, with names and roles. The editor uses text runs. | P0 | `tools/macos/ax_dump.swift` and `tools/windows/uia_probe.ps1` on Studio; Chrome's tree equals the kernel's (`web_a11y`-style); one hands-on pass per OS. |
| A11Y-3 | Zoom, reduced motion, no colour-only meaning | — | ⌘= / ⌘− UI zoom. Animations off under reduced motion. Diagnostics and state carry shapes and icons, not just colour. | P0 | Capture at zoom 150 %; the `prefers-reduced-motion` case shows no transition frames. |

### 3.15 Performance budgets (PERF)

Every row is measured in `tools/regress.sh perf` on the reference machines of [ide-plan.md](ide-plan.md) §6.2. All are P0.

| ID | Measure | Desktop | Web | Today |
|---|---|---|---|---|
| PERF-1 | Cold start → interactive Welcome | ≤ 1.0 s (warm ≤ 0.4 s) | ≤ 2.5 s first visit, ≤ 1.2 s cached | 479 ms; web 2.47–2.54 s first visit (measured with an uncompressed server: at the edge), 131 ms cached |
| PERF-2 | Open a 50-file project → first editor painted | ≤ 0.5 s | ≤ 1.0 s | +177 ms; web 151 ms |
| PERF-3 | Typing, key → pixels (100,000-line file) | p50 ≤ 8 ms, p99 ≤ 16 ms | p50 ≤ 16, p99 ≤ 33 ms | the model's share is 0.012 ms p99; the view isn't measured |
| PERF-4 | Completion popup | p95 ≤ 50 ms | p95 ≤ 100 ms | not measured in Studio |
| PERF-5 | Diagnostics after typing stops | ≤ 300 ms | ≤ 500 ms | Studio's Analyzer timer; not measured |
| PERF-6 | Toolbox add / inspector edit → designer + code | ≤ 16 ms | ≤ 33 ms | — |
| PERF-7 | Designer drag, 300 components | 60 fps, guides ≤ 2 ms | 60 fps | the model's guides < 2 ms; the view isn't measured |
| PERF-8 | F5 → the program's first form | ≤ 300 ms | ≤ 500 ms | not measured |
| PERF-9 | Run in Browser → form on the page | ≤ 1.5 s | — | — |
| PERF-10 | Build: 4 interpreted targets / web bundle | ≤ 10 s / ≤ 3 s | — | not measured |
| PERF-11 | Idle: CPU (caret blinking) / memory | < 1 % / ≤ 200 MB | < 2 % / ≤ 300 MB | not measured |
| PERF-12 | Design-time plot of a 100,000-row CSV | ≤ 300 ms | ≤ 1 s | — |

---

## 4. Polish (all P0 unless marked)

1. **Animations and transitions.**
   - The dock flyout slides in 120 ms; the palette and popups fade in 80 ms; completion appears without animation.
   - A designer drop settles in 100 ms; the "changed line" flash in the code fades in 600 ms.
   - Everything is instant under reduced motion. Nothing animates longer than 150 ms on a user action.
2. **Empty states.** Every pane says what to do and offers the action:
   - Properties: "Select a component on the form".
   - Outline: "Open a source file".
   - Problems: "No problems" with a check icon (done).
   - Output: "Press F5 to run".
   - Toolbox with no form: "Open a form to add components".
   - Project: "No project open — New… / Open…".
   - Search with no hits: what was searched, and a link to widen it.
3. **Error messages.**
   - Compiler messages in RapidQ's wording with a jump link.
   - Studio's own messages say what happened and what to do: "Can't run: 2 errors — the first is at line 12".
   - Never a raw Rust error or panic text.
   - The **`(… : not there yet)` fallback in `ide/shell.inc` `RunCommand` must be gone**: every menu command works or is hidden. Commands that fall through today: `edit.undo`, `edit.redo`, `edit.delete`, `edit.find`, `edit.replace`, `project.addForm`, `project.addModule`, `debug.toggleBreakpoint`, `debug.clearBreakpoints`, `debug.addWatch`, `debug.runToCursor`, every `designer.*` (Format), `help.contents`.
4. **Keyboard shortcuts per OS.**
   - Windows / Linux: the VB6 / Delphi scheme (F5, F9, F10, F11, F12, F2, F4, Ctrl+…).
   - macOS: ⌘ for Ctrl, plus Xcode's ⌘R run, ⌘. stop, ⌘B build, ⌘, settings, ⌘W close document, ⌘⇧[ / ] for documents, ⌘⇧O quick open, ⌘⇧L library.
   - A VS Code scheme is an option (P1).
   - Tooltips and menus show the OS's own glyphs (⌘⇧P, not Ctrl+Shift+P).
   - A test lists every command's shortcut per OS and finds no clashes.
5. **Drag feedback.**
   - A ghost of the component at 60 % opacity while dragging from the toolbox.
   - The target container is highlighted; snapping guides show distances.
   - A "not allowed" cursor over invalid drops; Escape cancels the drag.
   - The docking compass is labelled (done).
6. **Icons.**
   - Only RapidR's own D8 set (`design/icons`): every menu item, toolbar button, toolbox item, file kind and symbol kind.
   - The inventory test fails when one is missing (done for components and commands).
   - Hinted at 16, 24 and 32 px, crisp at every scale.
7. **Sounds.** None. Studio never beeps; errors and states are visual and announced.
8. **Dark mode.**
   - Every pane, dialog (New Project, the palette, message boxes), scroll bar, tooltip, menu and the welcome lockup.
   - The macOS title bar follows the theme; the web page's background follows it before the first paint (no white flash).
9. **Small things that add up.**
   - Tooltips with the shortcut.
   - The caret blink follows the OS setting.
   - The window position and size are restored.
   - An "unsaved changes" prompt on close (Save / Don't Save / Cancel, OS order).
   - The title shows the project and a dirty dot.
   - Autosave on the web (done: OPFS).
   - Crash recovery on the desktop (P1).

---

## 5. Counts

| Area | P0 | P1 | P2 |
|---|--:|--:|--:|
| Welcome and templates | 3 | 2 | 1 |
| Designer | 16 | 2 | 2 |
| Inspector | 6 | 2 | 0 |
| Toolbox / Library | 4 | 2 | 0 |
| Code editor and IntelliSense | 9 | 3 | 1 |
| Project tree | 2 + 1 (proposed) | 2 | 0 |
| Run | 5 | 0 | 1 |
| Debug | 6 | 3 | 1 |
| Build and package | 4 | 2 | 1 |
| Search / palette | 3 | 1 | 0 |
| Help | 2 | 2 | 0 |
| AI | 1 (proposed) | 3 | 1 |
| Themes | 3 | 1 | 0 |
| Accessibility | 3 | 0 | 0 |
| Performance budgets | 12 | 0 | 0 |
| **Total** | **79** | **25** | **8** |

Items split across P1 and P2 (WEL-5, PRJ-4) are counted under P1. The polish list (§4) is P0 on top of these.

---

## 6. Gap table: every P0 item today

The state is from the code at `development` @ `bb23d078` (2026-10-08), not from the plan:
- **done**: works in Studio on both hosts;
- **partial**: the model or a piece exists, but Studio doesn't use it or a part is missing;
- **missing**: absent.

Lanes in flight (the designer, the panels / inspector, the code editor, the theme and the font) will move rows; update this table as they merge.

| ID | State | Evidence |
|---|---|---|
| WEL-1 | partial | The Welcome page with Start, Recent and example cards works (`ide/panes.inc` `FillExamples`, `LayOutWelcome`; `tests/studio_shell.mjs`). There are no live thumbnails (cards show a file icon) and no ▶ Run on the cards. |
| WEL-2 | partial | (S-SHELL-2) Five templates in `TEMPLATES` (`gui`, `console`, `rapidq` — main.bas, compat on —, `data`, `mdi`), each compiled and run headless; File > New Project shows them as a gallery of cards (`ide/workspace.inc`). The cards have icons, not thumbnails of their forms, and the `templates` flow (create → run on both hosts) isn't written. |
| WEL-3 | partial | No setup dialogs; the theme follows the system (`ide_theme`). The "no network on first run" check doesn't exist. |
| DES-1 | done | S-DESIGN: each component drawn by the kernel's own from a design-time store; `tools/visual/designer_wysiwyg.py` 8 / 8 pixel-identical (notepad, hello_form; light, dark; 1×, 2×). |
| DES-2 | done | S-DESIGN: click, Shift / ⌘-click, rubber band, 8 handles, live readouts, nudges, Alt frees; every change the smallest edit (`studio_flows` `designer`). S-DESIGN-2: the handles and grips keep their size and reach at any zoom. |
| DES-3 | done | Drawn while dragging (S-DESIGN); S-DESIGN-2's `Guides` property and the `designer-guides` flow / capture (a guide held on both hosts). The 300-component budget is the model's unit test (perf stage not yet). |
| DES-4 | done | S-DESIGN / S-PANELS: the placing tool (click or draw), drag and drop with a 60 % ghost of the real component and a "not allowed" pointer elsewhere, Enter / double-click (`AddComponent`), Delphi names, typing writes the Caption; a drop settles in for 100 ms (S-DESIGN-2). |
| DES-5 | done | `Document::sync` edits as OnSourceEdit; code → designer at the analyzer's pause, and when the designer is shown; read-only banner on errors. S-DESIGN-2: one undo history with the code (DES-10). |
| DES-6 | done | `CreateHandler` (S-DESIGN, S-PANELS' `create_handler`); the caret lands inside. |
| DES-7 | done | Pins on the canvas and the form's edges / corner with the live preview (S-DESIGN); the model's 40 / 40 parity. |
| DES-8 | done | The Format menu (`Arrange`), one undo step each. |
| DES-9 | done | Dropping on a panel / group box / scroll box reparents, target highlighted (S-DESIGN). |
| DES-10 | done | S-DESIGN-2: one history per file across the designer and the code editor (OnSourceStep / SharedUndo / OnUndo; Studio's interim history until RCODEEDITOR's ApplyPatches / Undo land) — `designer-undo-interleave`, `-undo-all` (exact bytes), `-redo`. |
| DES-11 | done | Ctrl / ⌘ + C, X, V, D on the designer (CREATE text on the clipboard). |
| DES-12 | done | Tab / Shift+Tab, Esc to the parent, arrows, Enter, the live region (`StatusText`) announcing each change (S-DESIGN); a keyboard-only five-minutes run is still to script. |
| DES-13 | done | S-DESIGN-2: the menu editor on the form's own bar (Type Here, `&`, separators, submenus, ShortCut captured, Checked in the gutter, drag to reorder) as QMAINMENU / QMENUITEM CREATE blocks; the Tab-order editor (badges, click in order) writing TabOrder — `designer-menu`, `designer-taborder`. |
| DES-14 | done | Non-visual components in a tray under the form (S-DESIGN); S-DESIGN-2: the program's own top-level dialogs (notepad's OpenDialog / SaveDialog) too, inspected and edited in their own blocks — `designer-tray`. Links between tray items aren't drawn yet. |
| DES-15 | missing | There are no `RDATAFILE` / `RDATASOURCE` / `RDBGRID` in the registry (`crates/rapidr-lang/data`); RPLOT is drawn by the kernel (done: L-FRAME) but nothing is live at design time. |
| DES-16 | missing | "Preview in classic" is approved and waits for L-THEME (ide-plan L-SHELL "Next"). |
| INS-1 | partial | `Designer::inspect()` gives registry rows, values, `in_code` and `mixed` (L-DMODEL). Studio's Properties pane is a `QSTRINGGRID` listing the form's own CREATE assignments as text (`ide/panes.inc` `ShowProperties`, `ide/documents.inc` `ScanForm`). |
| INS-2 | missing | No editors; the grid is display-only. |
| INS-3 | missing | No Events tab. |
| INS-4 | partial | `inspect` returns common rows and `mixed` (model only). |
| INS-5 | partial | Every registry entry has docs (`crates/rapidr-lang/data/glossary`); not shown in Studio. |
| INS-6 | partial | `Designer::set_property(prop, None)` resets (model only). |
| TBX-1 | partial | D8 icons and Q names are done, but the list is hard-coded (`ide/panes.inc` `FillToolbox`: about 34 components out of 97 + library ones). |
| TBX-2 | missing | No search. |
| TBX-3 | missing | See DES-4. |
| TBX-4 | missing | No hover card. |
| ED-1 | partial | `crates/rapidr-editor` is done (undo tree, multi-cursor, search, folding, 11 languages, the bench). RCODEEDITOR is still the memo-based view (`crates/rapidr-ui-kernel/src/components/codeedit.rs`, `objects/{code,textedit}.rs`: one-step undo, no find / folding / multi-cursor). Nothing renders through `rapidr-editor` (only `rapidr-value/Cargo.toml` mentions it). |
| ED-2 | missing | Tab types a literal tab character (`codeedit.rs` header: "Tab typing a tab"); Robert saw it drawn as a stray glyph. No indent / outdent. |
| ED-3 | partial | `rapidr-langsvc` completes (golden suite, VS Code through `rapidr lsp`). RLANGUAGESERVICE exposes only `Update`, `Close`, `Outline` and `Diagnostics` (`crates/rapidr-studio/src/langsvc.rs`; ide-components §3.9), and RCODEEDITOR has no popup. |
| ED-4 | partial | Hover and signature exist in `rapidr-langsvc`; not exposed in Studio, no popup. |
| ED-5 | partial | Diagnostics fill the Problems list after a pause (`ide/documents.inc` `FillProblems`); no squiggles in the editor, no quick fixes in Studio. |
| ED-6 | partial | Definition, references and rename exist in `rapidr-langsvc` (and VS Code); not in Studio. |
| ED-7 | partial | `crates/rapidr-langsvc/src/case.rs` works in VS Code; not in Studio. |
| ED-8 | partial | Snippets in `rapidr-editor` (`snippet` module); not wired. |
| ED-9 | partial | The memo has IME; the editor's text runs for screen readers aren't there (I2 L-EDA11Y). |
| PRJ-1 | partial | Files with icons and double-click to open (`ide/project.inc` `FillProjectTree`, `ProjectTreeOpen`). Forms don't expand to components; no dirty marks in the tree. |
| PRJ-5 | missing | The old web IDE had an assets manager (its suites: §7); Studio has none. |
| PRJ-2 | partial | Add File works (`AddFileToProject`); `project.addForm` / `addModule` are "not there yet"; no rename or delete. |
| RUN-1 | partial | F5 saves and runs in its own process or a sandboxed frame; output goes to Output (`ide/project.inc` `StartProgram`; `tests/studio_flows.mjs` `run-console`). Output is a QRICHEDIT with no ANSI colours; there is no ⌘R. |
| RUN-2 | missing | No "Run in Browser" from desktop Studio. |
| RUN-3 | partial | Errors block the run and focus Problems (`StartProgram`); no jump to the first error. |
| RUN-4 | partial | The VM's break on error and the session's `Stopped` are done (I0 L-SESS); Studio only sets the status text (`ProgramStopped`), with no line highlight and no stack shown. |
| RUN-5 | done | Stop and Restart (`RunCommand` `run.stop` / `run.restart`; `RProgramSession` kills the child). |
| DBG-1 | partial | `Session.SetBreakpoint` exists (protocol: file, line, condition); `debug.toggleBreakpoint` is "not there yet"; no gutter markers. |
| DBG-2 | partial | Step In / Over / Out and Pause are wired (`RunCommand`); the editor doesn't follow or highlight the line; Run to Cursor is missing. |
| DBG-3 | missing | No Variables or Watch panel (the protocol has `scopes` / `variables`). |
| DBG-4 | missing | No Call Stack panel (the protocol has `stackTrace`). |
| DBG-5 | missing | No data tips. |
| DBG-6 | partial | `? expr` and statements work when paused (`ide/project.inc` `ImmediateKey`); no history and no completion. |
| BLD-1 | missing | `run.build` only checks that the program compiles (`ide/shell.inc` `BuildProject`). The CLI already builds interpreted executables per target (`rapidr build --interp --target`; runners in `crates/rapidr-cli/src/home.rs`). |
| BLD-2 | missing | Not from Studio (the CLI's `rapidr build` works). |
| BLD-3 | missing | Not from Studio (the CLI's `rapidr build --web` works). |
| BLD-4 | missing | No Project Options UI (`project.options` prints a line). |
| CMD-1 | done | The palette: commands, examples, `:N` (`ide/window.inc` `Palette`; `tests/studio_flows.mjs` `palette`). Fuzzy matching is to be checked. |
| CMD-2 | missing | No quick open for files, symbols or components. |
| CMD-3 | done | (S-SHELL-2) Ctrl+Shift+F: the Search pane (`ide/search.inc`), RPROJECT `Find` / `Replace` on the code editor's own search (case, whole word, regex), open documents searched as their editors have them, results by file linking to the line, Replace All (one edit a file). `tests/studio_flows.mjs` `find-in-files` (3 hits in a project and its include), both hosts. |
| HLP-1 | done | (S-SHELL-2) F1: the Help pane with the registry's entry (RLANGUAGESERVICE `Help`: title, syntax, kind / type / default / origin / runtimes, doc, a component's members) for the word at the caret (a member of `Name.`'s component), the inspector's row, the toolbox's item, the designer's component; Insert types the syntax. `help`, `help-f1` flows. No example in the entry (the registry has none yet). |
| HLP-2 | missing | No hovers in Studio (see ED-4, INS-5, TBX-4). |
| AI-1 | missing | No `rapidr mcp` (`crates/rapidr-cli/src/main.rs`). |
| THM-1 | partial | RapidR light, dark, high contrast and classic follow the system at start (`ide/commands.inc` `view.theme.*`); following a change live is open (ROADMAP Phase 1 themes item). |
| THM-2 | done | `ide_theme::editor` colours per theme (`crates/rapidr-value/src/objects/code.rs` `style_in`). |
| THM-3 | missing | The designer has no program-theme preview (placeholders; DES-1). |
| A11Y-1 | partial | F6, Ctrl+Tab and the palette are done (L-DOCK, L-SHELL); designing by keyboard is missing (DES-12). |
| A11Y-2 | partial | The dock, trees and lists have AccessKit / ARIA (`tests/studio_shell.mjs`: trees equal on both hosts); no hands-on screen-reader pass on Studio, no editor text runs. |
| A11Y-3 | missing | No UI zoom command, no reduced-motion handling. |
| PERF-1, PERF-2 | done | Measured (ide-plan L-SHELL results). The web's first visit is at the 2.5 s edge. |
| PERF-3 … PERF-12 | missing | Not measured in Studio. **`tools/regress.sh` has no `studio` or `perf` stage**: `tests/studio_shell.mjs` and `tests/studio_flows.mjs` aren't in the gate. |

**Tally of the 79 P0 items** (PERF-1 … PERF-12 counted as 12): 5 done, 36 partial, 38 missing.

### Top 15 gaps, in order

Ranked by what Robert hit first, then by what the first five minutes need.

1. **ED-1 / ED-2: RCODEEDITOR on `rapidr-editor`.** It fixes Tab, and brings real undo, find / replace, folding and multi-cursor.
2. **DES-5: wire `rapidr-designer`'s `Document` into Studio.** RDESIGNSURFACE gets source members and an `OnSourceEdit`; `ScanForm` is deleted.
3. **DES-2: select / move / resize in Studio persisting to code,** with DES-10 undo through the same history.
4. **DES-4 / TBX-1 / TBX-3: the toolbox from the registry adds components** (double-click, Enter, drag, draw).
5. **INS-1 / INS-2: the real inspector.** Registry rows with typed editors, writing the smallest edit.
6. **ED-3: completion in Studio.** RLANGUAGESERVICE gains `Complete` / `Hover` / `Signature`; RCODEEDITOR gains the popups.
7. **DES-6 / INS-3: double-click and the Events tab create handlers.**
8. **DES-1: WYSIWYG.** Real components drawn from a design-time store, equal to the run.
9. **DBG-1 / DBG-2 / RUN-4: breakpoints in the gutter, the current line followed and highlighted.**
10. **BLD-1 … BLD-3: Build for every OS and the web from Studio** (the CLI already does it).
11. **RUN-2: Run in Browser from desktop Studio.**
12. **Polish 3: no "not there yet" commands.** Undo, find, align, add form / module, help: each one works or is hidden.
13. **ED-5 / ED-4 / ED-6: squiggles, hovers, quick fixes, go to definition and rename in Studio.**
14. **DBG-3 / DBG-4: Variables / Watch and Call Stack panels** (the protocol is ready).
15. **The gate:** a `studio` stage (`studio_shell`, `studio_flows`, the new `studio_wow`) and a `perf` stage in `tools/regress.sh`, so none of this regresses.

Next after these: WEL-2 (the template gallery), CMD-2 (one search), HLP-1 (F1 help), DES-14 / DES-15 (the tray and live data, I7's core), DES-12 / A11Y (designing by keyboard, the screen-reader pass), and AI-1 (if Robert approves it as P0).

---

## 7. The old web IDE's suites (retired 2026-10-08)

Robert decided (2026-10-08) that RapidR Studio is the web IDE: the HTML / Monaco IDE and its folder were deleted. Its browser suites (`tests/web_ide_*.mjs` and a few others) checked three kinds of things:
- **the web runtime** (what a program does in the browser): re-pointed at the runtime's own page (`tests/web_run.mjs` over `tests/web_kernel.html`) or at Studio's run frame (`tests/studio_run_frame.mjs`), and kept in `tools/regress.sh web`;
- **IDE features**: a Studio flow where Studio has the feature (`tests/studio_flows.mjs`), otherwise the item below that will hold it (nothing is dropped silently);
- **the old IDE's own internals** (its DOM, its preview iframe, its Monaco): deleted with it.

| Old suite | What it checked | Now |
|---|---|---|
| `web_ide_align`, `_canvas`, `_components`, `_dialogs`, `_grid`, `_grid_draw`, `_lists`, `_objects`, `_owner_list`, `_picture`, `_reentrant_events` | programs' components on the web | runtime: `tests/web_align.mjs`, `web_canvas`, `web_components`, `web_dialogs`, `web_grid`, `web_grid_draw`, `web_lists`, `web_objects`, `web_owner_list`, `web_picture`, `web_reentrant_events` (same checks, on `web_run.mjs`) |
| `web_conformance`, `lang_conformance` (web), `web_sqlite`, `web_vm_yield`, `web_modal_focus`, `corpus_web_compare`, `_webprobe` | programs' output and behaviour | runtime: the same files, on `web_run.mjs` |
| `web_file_dialogs` | QOPENDIALOG / QSAVEDIALOG with the user's files | runtime + Studio: the program in Studio's run frame, whose page shows the browser's pickers for it (`ide/web/studio.js` `frameFiles`) |
| `web_ide_preview_isolation` | SEC-02 / 03: the program can't reach the IDE; storage per program; output once | Studio: `tests/studio_run_frame.mjs` (1, 2) |
| `web_ide_examples` | (1–2) every example loads into the designer / editor, local `.rr` / `.bas` open; (3) the run window at 1:1 pixels at DPR 1 and 2 | (3) runtime + Studio: `tests/web_pixels.mjs`, `studio_run_frame.mjs` (3). (1–2) Studio: examples open (`run-console`, `designer`, `outline-problems`); every example in the designer: **WEL-1 / DES-1** |
| `web_ide_debugger_test`, `debug_e2e_flow`, `debug_e2e_event_handling` | breakpoints (main code, an event handler), stepping, stack, variables, a watch, Stop | runtime: `tests/web_debugger.mjs` (the session protocol as Studio drives it; `web_session.mjs` for the protocol's own cases). Studio's UI for it (gutter, F9, Call Stack, Variables, Watch): **DBG-1 … DBG-4** (missing: `debug.toggleBreakpoint` is "not there yet", no panels) |
| `web_ide_console` | CLS / COLOR / LOCATE rendered in Output | Studio: `studio_flows` `run-ansi` (the text); the colours: `rapidr_value::panels::console::screen` tests; colours seen in a flow: **RUN-1** `run-ansi` capture |
| `web_ide_diagnostics` | errors listed, nothing runs with errors; a click jumps to the line; squiggles; live while typing | Studio: `studio_flows` `problems` (listed, nothing runs). Jump to the error, squiggles: **RUN-3, ED-5** |
| `web_ide_smoke` | Run an example: it runs, no page errors | Studio: `studio_flows` `run-console` |
| `web_ide_designer` | drop components → CREATE blocks; Caption edited in the grid; the result runs | Studio: `studio_flows` `designer`, `designer-add`, `toolbox-add`, `inspector-edits-code` |
| `web_ide_undo` | each designer / grid action one undo step; redo; a new edit clears redo; Delete + undo; code typing vs text undo; New resets history | Studio: `designer-undo`, `inspector-undo`. Code-editor undo grouping, Delete + undo, New resets: **DES-10, ED-1** |
| `web_ide_tree_validation` | components in the tree; tree ↔ designer selection; double-click → handler stub; a module name with spaces offered sanitized | Studio: `tree-and-search`, `inspector-event-handler`. Components under the form in the tree with selection sync: **PRJ-1**; double-click → handler: **DES-6**; Add Module's name check: **PRJ-2** |
| `web_ide_round3` | Object drop-down jumps to the handler; the form's properties when the form is picked; Visible / Enabled checked by default; a closed tab reopened from the tree | Object / Event bar: **ED-10** (P1); form properties and Boolean editors: `inspector-typed`, **INS-1 / INS-2**; reopen from the tree: **PRJ-1** |
| `web_ide_bugfixes` | move on the first click; real components in the designer; live property → designer; per-component events; copy / paste; About with credits and licence; Full Source; modules in the tree; zip dates; no black backdrop | **DES-2, DES-1, INS-2, INS-3, DES-11** (Studio has `edit.copy` / `edit.paste` on the designer, no flow case yet), About: Studio's `help.about` (no flow case); Full Source: dropped (Studio's source *is* the form, DES-5); modules: **PRJ-2**; zip dates: **BLD-3**; the backdrop: the old preview's own |
| `web_ide_phaseF` (failing) | (1–6) a second form, each form's components, the project saved as JSON and loaded back; (7) the restored two-form project runs | (7) runtime: `tests/web_multiform.mjs`. (1–6) Studio: `save-project` (one file); Add Form exists (`project.addForm`), a two-form project saved and reopened: **PRJ-2** |
| `web_ide_round4` (failing) | (1) two forms shown / focused across each other; (2–4) the colour picker and font list update the designer live, OK / dismiss; (5) the built zip's loader imports the runtime | (1) runtime: `tests/web_multiform.mjs`. (2–4) **INS-2** (colour, font editors). (5) runtime: `tests/web_bundle_*.mjs` (CLI bundles); from Studio: **BLD-3** |
| `web_ide_e2e`, `web_ide_e2e_build` | each example run in the IDE, then built to a zip and served: the same content; a built bundle's button works; manifest, CSP, notices | bundles: `tests/web_bundle_*.mjs`, `tests/examples_run.mjs` (web); building the bundle from Studio: **BLD-3** |
| `web_ide_assets`, `web_ide_assets_explorer` | the assets manager: add, list, filter, preview (image, text, CSV), rename (references follow), delete, pick for a Picture property; assets in the saved project and the zip | **missing in Studio**: an Assets pane (proposed **PRJ-5**, P0: the project's files with previews, rename updating references, picked by INS-2's picture editor; carried by `.rrproj` and BLD-3's bundle) |
| `_q`, `visual_smoke`, `capture_assets_explorer_screenshot` | ad-hoc probes and screenshots of the old IDE's DOM | deleted (old internals) |

`tests/studio_shell.mjs` and `tests/studio_flows.mjs` serve `target/studio-web` (port 18473); they are not in `tools/regress.sh` yet (§6, gap 15).

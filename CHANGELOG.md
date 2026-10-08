# Changelog

All notable changes to RapidR are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project uses [Semantic Versioning](https://semver.org/). Planned work lives in
[ROADMAP.md](ROADMAP.md); security finding IDs (`SEC-xx`) refer to it.

## [Unreleased]

### Changed
- **Real bold and italic faces** instead of the regular letters drawn heavier and slanted: Liberation
  2.1.5's own Bold, Italic and Bold Italic (Sans, Serif, Mono; SIL OFL 1.1, from the official release,
  SHA-256 checked) are built in, cut to the Latin scripts without hinting (36-39 KB each) and renamed
  "RapidR Text Sans / Serif / Mono" as the licence asks; RapidR Sans has a Bold of its own (MS Sans
  Serif Bold's widths, a pixel wider a character, and its two-pixel stems). Bold Arial text is as wide
  as Arial Bold's ("Pantry" 9 pt bold 37 pixels, "Hello" 12 pt bold 39, as RC.EXE measures), italic is a
  designed italic; a character the styled face lacks (Greek, Cyrillic, ...) is drawn from the Regular
  face, made bold or slanted, as before. The built-in fonts are now in a program once (statics, not
  copies of consts): the interpreter and the web runtime are 1.3 MB smaller in spite of the new faces.
  The approved images of fonts, words, spacing and the data-science examples changed with them
  (bold looked at beside RapidQ's, all themes, 1x and 2x, web identical to the desktop).

### Look
- **RapidR's look is every program's default** (`$THEME rapidr`, or no
  `$THEME` at all): RapidR Studio's own look, one set of tokens for Studio
  and programs, made from the brand palette (Ink, Paper, RapidR Blue, Blue
  Deep, Blue on Dark, Slate, Mist, Board; `docs/theme-tokens.md` maps every
  token). Light, dark (on the brand's Ink) and high contrast, following the
  system's setting — and its changes — unless the program names one;
  `Application.Theme` reads `rapidr light`, `rapidr dark`, `rapidr high
  contrast` or `classic`. RapidQ's default font is drawn in Inter at MS Sans
  Serif's 11 pixels (chosen by the clipping audit: 26 more of the RapidQ
  corpus' 539 fixed-size captions clip at 11 px, 56 at 12, 78 at 13); fonts
  a program names keep their face.
- Refined controls: 5-pixel corners on controls, 8 on menus, drop-down
  lists and windows; soft shadows under menus, drop-down lists and tooltips
  (a new `Op::Shadow`, the same on the desktop and the web) and around a web
  page's windows; buttons with a hairline rim a step darker along the
  bottom; one focus ring (2 pixels, RapidR Blue) on text boxes, buttons and
  combo boxes, a 1-pixel accent border on lists, trees and grids; the
  selection in the accent while a list has the focus, grey without it;
  grids with light lines, a tinted selection, the current cell ringed and
  text centred in its row; tabs as words underlined in the accent.
- **Sizes never change between themes**: an AutoSize label's and a tab's
  sizes are measured as RC.EXE measures them in every theme (the text runs
  on rather than being cut where Inter is wider). `tests/theme_geometry.mjs`
  runs every GUI program of `examples/` (and, with `--corpus`, RapidQ's own
  examples) in all four looks and compares every component's place.
- **`$THEME classic`** is RapidQ's look, and a web page's window frame in it
  has Windows' own 16 × 14 caption buttons (they were 26 × 23, huge at 2×).
  The web frame is now the kernel's (`window_frame`), as QFORMMDI's children
  are on every runtime. The classic names stay classic (`System`, `Light`,
  `Windows`, `Win95` …); `modern`, `dark` and `highcontrast` are RapidR's
  light, dark and high-contrast looks.
- RapidR Studio: **Preview in Classic** (View menu, tool bar, command
  palette; saved with the settings) runs the program and draws the designer
  in RapidQ's look without editing the source (`RPROGRAMSESSION.Theme`,
  `RDESIGNSURFACE.Theme`). Studio's status bar is quiet at rest and takes a
  state's colour while the program runs, pauses or fails.
- Text a program left uncoloured on a colour reads at WCAG AA in RapidR's
  look (a gauge's percentage white on the accent, the theme's text on the
  rest); RapidR Studio's panels and icons follow the theme's palette in
  dark and high contrast (they fell back to the light icons).
- RC.EXE pixel comparisons (`tests/native_gui_events.mjs`, the kernel's unit
  tests that check ops) name the classic theme explicitly; the visual
  gallery's themes are `classic`, `rapidr-light`, `rapidr-dark`,
  `rapidr-high-contrast`.
### Added
- **Serial ports for ESP32 / Arduino / IoT boards** (RComPort, RapidQ's QCOMPORT; RapidQ's members
  unchanged, RapidR's extras added, the same in native builds, the interpreter and the browser):
  **ListPorts** with each port's USB vendor / product IDs, description, maker and serial number
  (IOKit on macOS, SetupAPI on Windows, sysfs on Linux, Web Serial's granted ports in the browser;
  CP210x, CH340, FTDI, ESP32 USB JTAG/serial … named from their IDs where the system says nothing),
  **FillList** fills a list or combo box; the **DTR / RTS** lines set and read back, **CTS / DSR /
  CD / RI** read, **SendBreak**; **ReadLine**(Timeout) with a configurable **LineEnd** and
  **HasLine**, the **OnLine** event; **OnPortsChanged**(Added, Removed) when a USB adapter is plugged
  in or out. Closing a port in the browser now closes the Web Serial port (it can be opened again).
  Tested on a real ESP32 (an M5StickC Plus on FTDI): reset through DTR / RTS, its ROM boot log read,
  interpreted, native and in Chrome. Example: `examples/iot/esp32_monitor.rr`; manual: "Serial
  ports and IoT boards"; `tools/esp32_check.sh` checks a real board by hand (never writes to it);
  the scripted test ports gain `esp32` (a board that prints its boot log when reset).
- **RapidR Studio's panels as public components** (docs/ide-plan.md I1, L-PANELS), the same on the
  desktop, in native builds and on the web: **RPROPERTYINSPECTOR** (Delphi's object inspector: typed
  editors from the language registry, a visual Anchors pin editor, colours, fonts, lists, the Events
  page, defaults dimmed and reset, "RapidR extensions", multi-selection, a designer's selection
  followed), **RTOOLBOX** (RapidQ's and RapidR's components with RapidR's icons, search, drag and
  drop), **RPROJECTTREE** (.rrproj projects by kind, forms' components, rename, delete, reorder),
  **ROUTPUTCONSOLE** (ANSI output, build log, problems, `file:line` links, search), **RTOOLBAR** as
  a real toolbar (icon buttons, toggles, overflow, customizable) and **RCOMMANDPALETTE** (fuzzy
  commands). Example: `examples/studio/panels.rr`.
### RapidR Studio: documents as tabs, Find in Files, F1 Help, templates
- **No window inside the window**: Studio's documents are tabs, as in Xcode, VS Code and Delphi —
  close buttons, a dot for changes not saved, middle click closes, drag along the strip reorders. A
  file with a form is one tab with a **Design | Code** switch (and both side by side); F12 toggles,
  F7 / Shift+F7 pick. A tab dragged to a side of the documents splits them into **groups** (or
  Window ▸ Split Right / Split Down), with drop outlines and splitters. The layout, the open files
  and each file's view come back when the project is opened again; the window's place too.
- **RDOCKMANAGER** (tabbed documents): `AddView`, `DocumentView`, `DocumentModified`,
  `SplitDocument`, `DocumentGroupCount`, `OnDocumentView`; groups and views in `SaveLayout`.
- **QFORMMDI children resize by every edge and corner** (Windows' sizing border, resize pointers,
  never under Windows' 136 × 39 least size — read with RC.EXE on Windows 11), desktop and web.
- **Find in Files** (Ctrl+Shift+F): match case, whole word, regular expressions, results by file
  linking to the line, Replace All (RPROJECT `Find`, `Replace`, `FileText`).
- **F1 Help** pane from the language registry (RLANGUAGESERVICE `Help`) for the word at the caret,
  the inspector's row, the toolbox's item, the designer's component; Insert types the syntax.
- **New Project gallery**: Form app, Console, RapidQ-compatible form app (main.bas, compat on), Data
  dashboard, MDI app with menus. Closing a changed file asks Save / Don't Save / Cancel.
### RapidR Studio: the panels work
- **The real panels replace the stand-ins** (`ide/panels.inc`): the tool bar (RTOOLBAR, buttons with
  tooltips), the command palette (RCOMMANDPALETTE), the project tree (RPROJECTTREE), the toolbox
  (RTOOLBOX), the property inspector (RPROPERTYINSPECTOR) and the output console (ROUTPUTCONSOLE:
  output, build log, problems with links). Desktop and web byte-identical.
- **Properties work**: select a component on the form (or in the project tree, or by name in the
  palette) and the inspector shows its properties as its CREATE block writes them (defaults
  dimmed; Left / Top / Width / Height as laid out), by category or A–Z, searched, documented under
  the rows. A change is one undo step in the designer, written into the code as the smallest edit;
  the code edited reaches the inspector. Values are written as RapidQ needs them: Booleans as 1 / 0,
  and in a `.bas` program without RAPIDQ.INC a RapidQ constant as its number (RC.EXE reads an
  undefined `clRed` or `True` as 0).
- **Events**: double-click an event's row: its SUB is written with the registry's parameters (a
  DECLARE beside the file's, or the SUB before the form, as RC.EXE needs) and bound, or found; the
  caret goes inside it. The Events page offers the file's SUBs that fit.
- **Toolbox**: the registry's components by group with RapidR's icons; search also by what a
  component is ("chart" finds RPLOT); a card as the tooltip; double-click or Enter adds, a click arms
  the placing tool, a drag drops on the form.
- **Project tree**: forms with their components (selecting one selects it in the designer), files
  with changes marked with a dot, Project > Add Form / Add Module, rename (F2), take out of the
  project, reorder.
- **One search** (Ctrl+P, Ctrl+Shift+P): commands, examples, the project's files, the file's
  symbols and its form's components; `:N` goes to line N (the palette's AddPrefix).
- Empty panels say what to do (`EmptyText`).
- On the web an RTOOLBAR starts 32 pixels high, as on the desktop.

### RapidR Studio: the form designer works
- A file's [Design] tab shows its form as the running program shows it, at its own size, read from the code (`RDESIGNSURFACE.Source` on rapidr-designer's Document); the stand-in scanner and its boxes are gone.
- Drag the form's right edge, bottom edge or corner to resize it: anchored and aligned components follow live, and the new size (and where the anchors moved them) is written into the CREATE block, one undo step.
- Add components: click a type then click or draw on the form (`PlaceType`), drag one in (`DragComponent`), or `AddComponent(Type, X, Y)`. Names are unique in the whole file; typing right after adding sets the Caption; dropping on a panel puts it inside.
- Select, move, resize, delete, copy / paste / duplicate, align and arrange with the mouse or the keyboard; every change is announced to screen readers, written into the code as the smallest edit (`OnSourceEdit`), and undone to the exact bytes. Double-click makes or finds the event handler (`CreateHandler`). While the code has errors the designer is read-only under a banner.
### Removed: the old HTML / Monaco web IDE
- **RapidR Studio is the web IDE** (Robert's decision, 2026-10-08): the old HTML / Monaco IDE (`web-ide/`, ~10,000 lines of JS / HTML / CSS plus the vendored Monaco) is deleted, with `rapidr lang export --web-ide` and its generated `lang-data.js`, the brand export's copy of the icons into it, and every reference (regress, docs, release scripts, `tools/lang_seed.py`). Nothing shipped it any more (SEC-17).
- What a program's run frame needed from that IDE's page is Studio's now (`ide/web/studio.js`): the browser's Open / Save pickers shown for the sandboxed frame (writes only to files the user picked during that run) and the program's RWEBSTORAGE kept per program (1 MB). Studio's web page has RapidR's icons (favicon, home-screen icon).
- Its browser suites, sorted (docs/studio-wow.md §7): the web runtime's run programs on the runtime's own page (`tests/web_run.mjs`: `web_align`, `web_canvas`, `web_components`, `web_dialogs`, `web_grid`, `web_grid_draw`, `web_lists`, `web_objects`, `web_owner_list`, `web_picture`, `web_reentrant_events`, and the conformance, SQLite, VM-yield, modal-focus and corpus runners) or in Studio's run frame (`tests/studio_run_frame.mjs`: isolation, storage, 1:1 pixels); new `web_debugger`, `web_multiform`, `web_pixels`; new Studio flows `problems` and `run-ansi` (CLS / COLOR / LOCATE in Output). IDE features Studio doesn't have yet are listed against their checklist items (the assets manager is the new PRJ-5); the old IDE's own probes are deleted. `console_ansi` passes on the web (no xfail).
- Studio: Build with errors shows Output's Problems page (it named a pane that no longer exists).
### Security
- **Every web page RapidR builds has a Content-Security-Policy**, made from
  what the program uses (SEC-15): no inline scripts, `'unsafe-eval'` only for
  RJAVASCRIPT, frames only for RWEBVIEW, other servers only for the network
  components and the addresses the program spells out; `rapidr build --web
  --csp "…"` (and `bundle-bc --csp`) adds the author's own. Builds also write
  `_headers` and `.htaccess` with the same policy, `frame-ancestors 'self'`,
  `nosniff`, COOP and CORP. A program showing someone else's markup through
  RDOM no longer runs its scripts.
- **RWEBVIEW's page runs at an origin of its own** (SEC-12): the frame's
  default sandbox leaves `allow-same-origin` out (`Sandbox` opts in), and its
  `Html` runs in `rapidr-webview.html`, a frame every web build ships, with
  its scripts working as before.
- **Names in built pages are escaped for where they go** (SEC-16): a project
  or file named like markup or script is text in the page; no name is
  written into a script any more.
- **The release's web bundle is RapidR Studio** (SEC-17): `tools/release/web.sh`
  ships `tools/build_studio_web.sh`'s site; programs' bundles no longer carry
  any of the old web IDE's files.
- **`rapidr lsp` reads only the workspace** the editor opened (and the
  folders of the files it opened); **`rapidr dap`** launches only RapidR
  programs and never passes loader variables (`LD_PRELOAD`, `DYLD_*`,
  `PATH`, …) to them (SEC-18). The authentication `rapidr mcp` will need is
  designed (docs/security-audit.md §7).
- Supply chain (SEC-20): memmap2 0.9.11, anyhow 1.0.104; cargo-deny fails on
  unsound crates; `cargo deny` and `cargo audit` clean. Signing the release
  checksums with minisign is proposed (docs/security-audit.md §8).
- A `security` stage in `tools/regress.sh`, with a regression test for each.

### Legal
- **A review of RapidQ's terms, rights and trademarks, and of everything
  RapidR takes from RapidQ** (`docs/legal/rapidq-review.md`): RapidQ's
  freeware terms (free use, programs may be sold; only selling RapidQ itself
  is forbidden), the rights sold to REAL Software (now Xojo, Inc.) in 2000,
  trademark searches (no RapidQ or RapidR mark for software anywhere
  searched), the law on re-implementing a language and its interface, and
  the use of RC.EXE as a black box for testing. Every blob of the git
  history was compared with RapidQ's distribution and manual: no file of it
  was ever committed.
- The built-in `RAPIDQ.INC` constants (what `$INCLUDE "RAPIDQ.INC"` gives
  without the file) are regrouped by public origin — Windows SDK numbers,
  Delphi VCL types, RapidQ's own — in RapidR's own order and words; the
  same 483 names and values, pinned by a test. Programs see no change.
- What still came from RapidQ material is rewritten: a 9-line routine from
  one of RapidQ's examples in a conformance case, the manual's example data
  in three test fixtures and a unit test, and manual quotes in comments and
  docs (paraphrased). RC.EXE's output of RapidQ's own example programs moved
  out of the repository (`.reference/rapidq_golden/`, `RAPIDQ_GOLDEN`).
- `CONTRIBUTING.md` (no RapidQ code, text, includes, examples or media; what
  may be used for compatibility), `NOTICE` (shipped in every package),
  `docs/legal/clean-room.md` (how RapidR is developed independently), and a
  clearer non-affiliation and trademark statement in `LEGAL.md`, the README
  and the release notes.

### Added
- **Every build is an app for its system, with an icon** (`rapidr build`,
  `crates/rapidr-package`, docs/manual/building-apps.md). A program with
  windows becomes `Name.app` on macOS (Info.plist with its name, bundle ID,
  version, `NSHighResolutionCapable`, `LSMinimumSystemVersion`; its icon as
  `.icns` from 16 to 1024 px; signed ad hoc so it opens on Apple silicon;
  an interpreted program's bytecode in `Contents/Resources`), an `.exe`
  with its icon (16 to 256 px) and version information on Windows, and
  `Name.AppDir` on Linux (desktop entry, hicolor icons 16 to 512 px;
  `rapidr install-app` puts it in the applications menu). Console programs
  stay plain executables (`--bundle` makes apps of them; `--no-bundle`
  keeps any program a plain executable). Native and interpreted builds
  alike, cross builds too: the resources are written in pure Rust (editpe,
  BSD-2-Clause), so a Mac makes a Windows `.exe`'s icon.
- **The icon**: `--icon`, the project's `[build] icon` (`.rrproj`, with
  `app_name`, `bundle_id`, `version`, `company`), `$OPTION ICON`, RC.EXE's
  `-g<icon>` — in that order — as `.icns`, `.ico`, `.png` or `.svg`, made
  into every size each system wants (clear errors for files that aren't
  pictures, a note for small ones). Without one, RapidR's new **program
  icon** (`design/brand`: the Runtime's Ink tile, an app window, the amber
  Run triangle; `RapidR-App.icns`, `rapidr-app.ico`, `apps/rapidr-app`).
  `rapidr build app.rrproj` builds a project's main file with its settings.
- **RapidR Studio: Run > Build makes the app** for the system Studio runs on
  (RPROJECT.Build: `rapidr build` in the background, its lines in Output),
  **Run > Reveal in Finder / File Explorer / Files**, and **Project >
  Project Options**: the app's name, bundle ID, version, company, icon (with
  a preview) and native or interpreted build, saved in the project
  (`ide/build.inc`; RPROJECT's Icon, AppName, BundleID, Version, Company,
  BuildKind, Building, BuiltPath, FileManager, Build, StopBuild, Reveal,
  IconPreview, OnBuildOutput, OnBuildDone).
- `tools/studio_app.sh`: RapidR Studio as `RapidR Studio.app` from a
  checkout (its icon in the Dock, this checkout's Studio).
- **Every RapidQ member now works in RapidR.** The 101 RapidQ properties,
  methods and events that RapidR didn't answer before now work in native
  builds, the interpreter and the web. Each was checked against RapidQ's own
  compiler, RC.EXE:
  - **Streams.** QFILESTREAM has ReadByte and WriteByte. ReadByte gives 26
    past the end, as RapidQ does. QMEMORYSTREAM has `SetSize = n`,
    MemCopyFrom and MemCopyTo (with VARPTR or Pointer addresses). CopyFrom
    reports a stream read error when it asks for more bytes than are left.
    SaveUDTArray and LoadUDTArray are accepted and do nothing, as in RapidQ.
  - **String lists and rich edits.** QSTRINGLIST.LoadFromStream splits lines
    at CR LF, LF or CR. Its SaveToStream reads the list from the stream,
    exactly as RapidQ's does. QRICHEDIT has LoadFromStream and SaveToStream.
    QSTRINGGRID has DeleteColumn, another name for DeleteCol.
  - **Drawing.** TextRect draws text clipped to a rectangle on forms, images,
    canvases, bitmaps, headers, the printer, and owner-drawn lists and grids.
    Rotate turns an image, canvas or bitmap by degrees. QCANVAS has Get and
    Put, QIMAGELIST has Draw, and QIMAGE has Repaint. Lists, combo boxes and
    grids take RoundRect, CopyRect, StretchDraw, TextOut, TextWidth and
    TextHeight in their owner drawing.
  - **Forms.** HideTitleBar and ShowTitleBar remove and restore the title
    bar. ShapeForm cuts a window to a bitmap's outline (not on Wayland). A
    QFORM has Cascade, Tile, Next, Previous and ArrangeIcons. QFORMMDI has
    ActiveNextChild and ActivePreviousChild.
  - **Events that never fired now do.** OnHint fires when the mouse brings a
    new hint, and RapidR now shows tooltips (ShowHint, HintPause,
    HintColor). OnEnter fires when a list box or file list box gets the
    focus. OnStartDrag and OnEndDrag fire around a button drag, and
    StartDrag moves the control with the mouse. QCOMPORT fires OnBreak,
    OnRing and OnTxEmpty. A control's WndProc can be bound but is never
    called, as in RapidQ. A form's WndProc still gets its tray icon's
    messages.
  - **QMYSQL.** RealConnect, CreateDB, DropDB, Refresh, FetchLengths (with
    Length), RowBlob, LoadBlob and SaveBlob work. An empty host means this
    machine, as in RapidQ. EscapeString escapes the way MySQL's C client
    does.
  - New conformance cases (most with RC.EXE's own output as the expected
    output) and new GUI cases for the desktop and the web.
- **RapidR Studio: the IDE's shell** (`ide/`, docs/ide-plan.md I1 /
  L-SHELL + L-WEB). One RapidR program on RapidR's public components — the
  same bytecode on the desktop (`rapidr ide [file]`) and in the browser
  (`tools/build_studio_web.sh` → `target/studio-web`, drawn by the UI
  kernel on a canvas); their captures are byte-identical and their
  accessibility trees equal (`tests/studio_shell.mjs`: two scenes, four
  themes, 1× and 2×).
  - **The window:** a menu bar, a tool bar of RapidR's icons and the
    command palette (Ctrl+Shift+P), all made from one command table (VB6 /
    Visual Studio keys: F5 Run, Ctrl+F5, Shift+F5 Stop, F9, F8 / F10 /
    F11, Ctrl+S, Ctrl+O, Ctrl+N); RDOCKMANAGER with Project / Toolbox,
    Properties / Outline and Output / Problems / Immediate around an MDI
    documents area (tabs one click away); a status bar that turns green
    while the program runs and orange while it's paused.
  - **What it does:** opens a project, a source file with what it
    includes, or a folder; makes new projects; saves files and the
    `.rrproj`; runs and stops the program — in its own process on the
    desktop (its forms real windows), in a sandboxed frame on the web — with
    its output in the Output pane; shows each file's outline and the
    compiler's problems as you type; the Immediate window evaluates in a
    paused program; theme switch, MDI or tabs, the dock's layout and recent
    projects kept per user; `--do` runs commands (`rapidr ide --do
    run.start file.rr`).
  - **On the web:** the File System Access API for files and folders (saved
    back to the user's disk), Studio's own files kept in the browser's
    private file system between visits, and F5 / F6 / Ctrl+Tab as the
    IDE's keys.
  - **RapidR's look** is Studio's default — as the system is: light, dark
    or high contrast (`Application.Theme = "rapidr"`, `"rapidr light"`,
    `"rapidr dark"`, `"rapidr high contrast"`); classic stays one click
    away.
- **RPROJECT, RLANGUAGESERVICE, RPROGRAMSESSION** — RapidR Studio's
  services as public non-visual components (`crates/rapidr-studio`), the
  same on every runtime: a project (`.rrproj` v2 or v1, a source file and
  its includes, a folder), the language service's outline and diagnostics,
  and a run of a program under development (start, stop, pause, step,
  breakpoints, evaluate; OnOutput, OnStopped, OnExit …).
- **Tooltips**: a component's `Hint` shows in a small box when its
  `ShowHint` is True — after half a second, under the mouse or under a
  component the keyboard reached — as in RapidQ; drawn by the UI kernel on
  every host.
- **Inter and JetBrains Mono** built in (SIL OFL, Latin subsets): RapidR's
  UI and code faces — the RapidR look's menus, MDI titles, tooltips and
  code editor; programs can name them.
- `Application.ThemeColor(Name)`: a colour of the current theme by name —
  its own tokens and an IDE's (tool bar, status bar per run state, start
  page, editor colours).
- `QOPENDIALOG.PickFolder` chooses a folder; `QSTATUSBAR.Color`;
  `QTREEVIEW.ItemHeight` (QOUTLINE's) sets the rows' height;
  `RDOCKMANAGER.DocumentState`; a tab in a menu caption shows key text
  without binding it (Windows' menus' rule).
- **RapidR Studio's form designer model (I4, L-DMODEL), and anchoring that
  is the running program's.** The designer reads a program's
  `CREATE … END CREATE` blocks into a component tree (parents, z-order,
  Tab order), changes it with undoable commands (add, delete, move, resize,
  align, distribute, same size, bring to front / send to back, Tab order,
  cut / copy / paste, duplicate, rename, anchors, any property from the
  inspector), and writes each change back as the smallest text edit:
  a property's value, a new line in the block's own indentation, a nested
  CREATE before its parent's `END CREATE` — comments, blank lines, other
  code and everything outside the blocks stay byte for byte, and Undo
  gives the exact bytes back. Every form of `examples/` and of RapidQ's 386
  examples opens and saves unchanged; moving a component 8 pixels changes
  exactly one value. Where components are comes from the runtimes' own
  layout (Align, Anchors, Constraints, a QLABEL's AutoSize, scroll bars,
  the default sizes): dragging the designed form's corner previews it at
  another size with every component where the running program puts it —
  checked against the program itself resized, native, interpreted and in
  the browser (`tests/fixtures/designer_anchors.bas`).
- **RDESIGNSURFACE on the designer model**: Shift / Ctrl+click and a rubber
  band select several components; drags snap to the grid and to smart
  guides (siblings' edges, centres and text baselines, the form's centre
  lines, 8-pixel margins, equal spacing — Alt for none); eight handles; round
  anchor pins on the selected component's sides (click to anchor that
  side); the form's corner drags the resize preview; moves and resizes are
  one undo step each. New members: `Undo`, `Redo`, `AlignSelection(How)`,
  `SelectAdd(Index)`, `SelCount`, `PreviewWidth` / `PreviewHeight`,
  `ShowGuides`, `SnapToGrid`, `GridSize`. Its chrome follows the theme.
  `examples/form_designer.bas` shows it. Existing programs' calls answer as
  before.

- **An RPLOT on a form shows its chart, the same on the desktop and the
  web.** Put an RPLOT in a form's CREATE block and the chart is drawn in
  its place, filling its Left / Top / Width / Height, and follows Align
  and Anchors as the form is resized. It is sharp at any screen scale
  (1×, 1.5×, 2×) and drawn in the program's theme (classic, modern,
  dark). It is drawn again whenever the chart changes: a new series
  (`Plot`, `Bar` …), a property (`Title`, `Grid` …), `Render` / `Show`.
  Native builds, interpreted programs and the browser show the same
  pixels, and a screen reader hears an image named by the chart's Title.
  On the web the chart is no longer a separate page element: it is drawn
  on the form like every other component (the old element is gone). A
  chart that is never placed on a form works as before (`SaveFig`,
  `Image.LoadFromPlot`). A new RPLOT is 640 × 480, the chart's own size,
  everywhere.
- **Automatic keyword case in VS Code, as in QuickBASIC and VB.** Type
  `dim x as integer` and it becomes `DIM x AS INTEGER` as you go: each
  word is put in BASIC's case when you finish it (space, Enter, Tab, `(`,
  `)`, `,`, `:`, `=` or an operator) — keywords and statements, type
  names, directives (`$INCLUDE`) and builtins (`MID$`). Strings, comments,
  `$INCLUDE` paths and your own names are never touched, and one Undo
  gives back what you typed. Format Document and Format Selection apply
  the same rules to the whole text. Two settings:
  - `rapidr.keywordCase`: `upper` (the default), `lower`, `proper`
    (`Dim x As Integer`) or `preserve` (off);
  - `rapidr.identifierCase`: `declaration` writes every use of your
    variables, SUBs, FUNCTIONs and constants as their declaration does,
    and a component's members as RapidR spells them (`Form.Caption`), as
    VB did; `preserve` (the default) leaves them as typed.

  BASIC ignores case, so a program never changes: a test formats every
  conformance program and example and checks that it compiles to the same
  code, its strings and comments byte for byte. Format on type is now on
  by default for RapidR files (`editor.formatOnType`). RapidR Studio's
  editor will use the same rules.
- **The language registry** (`crates/rapidr-lang`, RapidR Studio I0): one
  description of the language (every component, property, method, event,
  builtin, statement and constant, with types, defaults, signatures and
  short docs in RapidR's own words), compiled from `data/*.toml`. The
  compilers' component list, the runtimes' name tests, the editor's
  keyword groups, the icon tools, the manual's reference pages and the
  web IDE's completion data all come from it; `rapidr lang export` writes
  them, `tools/lang_dispatch.py --check` proves every name the runtimes
  answer is in it, and `tests/lang_conformance.mjs` runs one program per
  component on every runtime. RDOCKMANAGER and the icon methods
  (`Bitmap.LoadIcon`, `ImageList.AddIcon`) are in it.
- **The user manual** (`docs/manual/`): getting started, the language,
  components (RapidQ's Q names and RapidR's R names), the CLI and the
  Runtime, the web, databases, data science, DirectX and media,
  differences from RapidQ, troubleshooting. Its reference pages (components,
  builtins, members, statements, constants, data-science members) are
  generated from the language registry by `rapidr lang export --manual`
  (`cargo test -p rapidr-lang` fails when they're stale).
  Every SDK installs it in `share/doc/rapidr/manual/`.
- The release notes of v2.117.0, the first public release
  (`docs/release-notes/v2.117.0.md`).
- Conformance cases `datascience_num`, `datascience_frame` and
  `datascience_plot`: every documented RNUM, RDATAFRAME and RPLOT member,
  checked on the VM, native builds and the web (`tests/web_conformance.mjs`).
- Data-science members the desktop or the web lacked now work everywhere:
  RNUM `create`, `set`, `get`, `push`, `avg`, and `Sum` / `Mean` / `Min` /
  `Max` / `Std` / `Count` as properties; RDATAFRAME `create`, `addrow`,
  `savetocsv`, `loadfromjson`, `savetojson`, `iloc`, `sort_values`,
  `query`, `groupby`, `value_counts`, `nunique`, `corr`, `nlargest`,
  `nsmallest`, `dtypes`, `merge`, `concat`, `apply`, `replace` on the web;
  RPLOT `addseries`, `settitle` / `setxlabel` / `setylabel`, `show` /
  `render` on the desktop, and `xscale` / `yscale`, `xlim` / `ylim`, `DPI`
  on the web. `LoadFromCSV` / `LoadFromJSON` also take the data itself;
  `filter` also takes `=`, `<>`, `startswith`, `endswith`, and `contains`
  is a substring test on every runtime; `groupby` also takes `median` and
  `std`; `cell` / `setcell` also take a column name.
- **The examples, curated for the first release** (`examples/`, indexed by
  `examples/README.md`): 26 small commented programs by topic — basics/
  (hello, INPUT, files, the language), gui/ (a form and its events, menus,
  dialogs, a timer, a list and a grid, themes, the tray icon), graphics/
  (canvas), directx/ (QDXSCREEN sprites, a Direct3D cube from a `.X`
  model), media/ (QMIDI with the built-in synthesizer, QWAVE, QVIDEO), data/
  (SQLite with parameters, JSON, RNum, RDataFrame and RPlot), network/
  (RHTTP + RJSON, QDOWNLOAD), rapidq/ (a notepad in plain RapidQ code),
  web/ (browser storage, RJAVASCRIPT and RROUTER) — plus the IDE. All of
  them RapidR's own; their media and models are made by
  `tools/make_example_media.py` (nothing downloaded). The old test scraps
  and demos went (and the scripts that drove them: `tools/native_examples.sh`,
  `tests/full_matrix.sh`, `tests/bc_smoke.sh`, `tests/web_matrix.mjs` and the
  demos' ad-hoc page scripts); the web IDE's examples list shows the new ones,
  with the files their `$RESOURCE` lines name.
- `tests/examples_run.mjs` (the `examples` stage of `tools/regress.sh`, in
  place of `tools/native_examples.sh`'s `cargo check`): every example RUNS on
  every runtime it claims — `rapidr run`, interpreted and native executables
  (each native build dropped after its run), the web (the UI kernel's page)
  — GUI ones through the test hooks (events, then the components' properties
  read back), network ones against the tests' own local server; the table
  must list every program and the README every entry.
- `rapidr examples` lists the examples and `rapidr examples copy <name|all>
  [folder]` copies one (with the data files it names) or all of them; SDK
  installs ship them in `lib/rapidr/examples/` (the home's, as a checkout's
  `examples/`: `tools/release/stage.py`).

- **RapidR's own icon set** (IDE plan decision D8; `design/icons/`). There
  are 344 icons, our own drawings (MIT), each drawn once on a 24 px grid and
  hinted at 16, 24 and 32 px:
  - every action and command of RapidR Studio;
  - every component type (QBUTTON and RBUTTON share one), the IDE plan's
    planned components and RPLOT's chart kinds;
  - file types (the brand's R on `.rr`, `.rrbc` and `.rrproj`), symbol
    kinds, toolbox groups and glyphs.

  Monochrome action icons follow the theme; component icons are two-tone,
  in colour tokens for the classic, modern, dark and high-contrast themes
  (3:1 or better everywhere, monochrome in high contrast). The design system
  is in `design/icons/README.md`.
  - The new crate `rapidr-icons` themes and renders the icons through
    resvg, picks the drawing hinted for the device's size (crisp at 1×,
    1.5× and 2×) and builds for wasm32. Its SVGs are stored deflated
    (42 KB) and inflated on first use.
  - The UI kernel draws an icon with `Painter::icon`.
  - New in programs (additions; RapidR's own): `Bitmap.LoadIcon(Name$
    [, Size [, Theme$]])` and `ImageList.AddIcon(Name$ [, Theme$])`, by
    icon name (`"run"`), component type (`"QBUTTON"`) or command id
    (`"file.open"`).
  - `design/icons/inventory.toml` lists what has an icon. Components come
    from `COMPONENT_TYPES`; the toolbox groups keep RapidQ's components
    under "RapidQ" and the rest under "RapidR". The build and the crate's
    tests fail when a component type or a command has no icon.
  - The manual's component reference shows each component's icon, and
    `docs/manual/icons/` holds every icon as SVG (light and dark), PNG and
    an HTML catalog.
  - `tools/regress.sh unit` checks that everything is up to date.

### Changed
- **`$OPTION ICON` as RapidQ's compiler has it** (checked against RC.EXE):
  an icon file that isn't there is a compile error, `ICON file x does not
  exist.` (it used to leave the default icon); the file name may be
  unquoted; with several, the last one wins. The icon is now also the built
  executable's / app's. RapidR still takes any `.ico` (RapidQ only 766-byte
  32 × 32 ones), `.icns`, `.png` and `.svg`.
- QDXSCREEN.TextRect now uses the same drawing as every other TextRect: a
  background colour fills the whole rectangle, not just the text.
- `SetFocus` on the desktop now moves the keyboard focus, as it already did
  on the web.
- **`rapidr ide` opens RapidR Studio** (`ide/studio.rr`; an install's
  `ide/rapidr-ide.rrbc` is compiled from it); the old `examples/ide.rr`
  and the HTML web IDE stay until Studio reaches their features
  (docs/ide-plan.md I1's acceptance).
- **RapidR's look (modern, dark, high contrast) draws its chrome in its own
  fonts and shapes**: menus, MDI titles and tooltips in Inter; the code
  editor in JetBrains Mono on the theme's editor colours (keywords readable
  in the dark look); MDI windows rounded with a light title bar; the dock's
  headers 28 pixels with their buttons where the mouse or the focus is;
  flat tool buttons a soft rounded fill under the mouse; the menu bar's
  access keys underlined from the keyboard only. The classic look and every
  program's own metrics are unchanged.
- **The dialogs the UI kernel draws** (message and input boxes, the colour
  and font dialogs) use the look's chrome font: Inter in RapidR's look,
  MS Sans Serif in the classic one (unchanged). RapidR Studio's own
  dialogs follow it, with fields and buttons sized for it.
- RDOCKMANAGER's headers are 28 pixels in every look (a pane's inside is
  2 pixels shorter than before).
- **On a Mac, a menu's `Ctrl+` ShortCut is ⌘** (the system menu bar's
  key equivalent; in-window menus take either), as Mac programs' keys are.
- A flat button whose only glyph is a picture (no disabled frame) is drawn
  greyed while disabled, as Windows greys a speed button's glyph.
- RCODEEDITOR reads a UTF-8 source as UTF-8 (`LoadFromFile`) and writes it
  back so; other files stay a byte a character, as RapidQ's text boxes.
- **One layout sequence for every runtime.** When Align and Anchors place
  components (a property set, a container resized) is
  `rapidr_value::layout::engine`'s, run by the desktop and web runtimes
  and by the designer; nothing a program sees changes.
- The language registry's component sizes are the runtimes' (RapidQ's
  measured defaults); a test keeps them the same.
- **The web answers every member the desktop does, and members RapidQ
  doesn't have are refused as RapidQ refuses them.**
  - QSTATUSBAR `Clear` removes every panel (panels added afterwards start
    again at `Panel(0)`, as RapidQ does); QMEMORYSTREAM `Clear` empties the
    stream (`Size` and `Position` 0, as RapidQ). Both did nothing before,
    and the browser didn't know them.
  - QIMAGE `Clear` / `Cls` remove the picture and `Load` loads one, as
    `LoadFromFile` does, in the browser too.
  - RapidQ's drawing methods on lists and grids draw what RapidQ draws
    (checked against programs built by RapidQ's own compiler): `Paint`
    flood-fills in an owner-drawn list box's, combo box's or grid's
    handler; `Line`, `FillRect`, `Circle`, `Paint` and the rest on a list
    box that isn't owner-drawn draw on the list, which keeps the drawing
    until it paints those rows again; on a combo box that isn't
    owner-drawn nothing shows. Pixel for pixel the same in the browser and
    on the desktop, at normal and high-DPI scale.
  - `Click`, `SetParent`, and other names RapidR's desktop runtime used to
    accept on any component (`AddItem` on a list view or popup menu,
    `AddItems` on an edit, `Rect`, `SetPixel`, `Ellipse`, `DrawText` on a
    list, grid, bitmap or DirectX screen, `Clear` on a bitmap …) are now
    the compile error RapidQ gives: "Member CLICK not part of class BTN".
    They did nothing, or nothing useful, before.
  - RSERVERSOCKET stays desktop only: a web page can't listen for network
    connections.

- **The VS Code extension and RapidR Studio know the language from the
  language registry.** Completion, hover, signature help and the
  compatibility warnings come from the one description of the language
  RapidR's compilers and IDE use (the language service's own copy of it is
  gone). What you see:
  - hovers give a property's type, whether it is read only and RapidQ's
    default, the include file a RapidQ name comes from, and say when
    something is a RapidR extension, works on the desktop or the web only,
    or isn't implemented in RapidR yet;
  - `Screen.`, `Application.`, `Printer.` … complete and hover, and so do
    the objects an indexed property gives (`Tree.Item(0).`);
  - what RapidR doesn't have yet isn't offered in completion, and where a
    program uses it there is a warning ("RapidQ's QForm.ShapeForm is not
    implemented in RapidR yet"); a member or builtin only one runtime has
    gets a note;
  - in a RapidQ-compatible project (`rapidr.rapidqCompatible`), every
    RapidR extension is reported, not only components: members of RapidQ's
    components (`QForm.Anchors`), builtins, statements (`OPEN`,
    `LINE INPUT`, `PRINT #`), the `$THEME` directive, types (`INT64`) and
    RapidR's constants — "… is a RapidR extension: RapidQ's compiler
    refuses it";
  - signature help for builtins shows their syntax with the parameter you
    are typing highlighted, also for statements written without
    parentheses (`LOCATE 1, 2`).
- **RNUM, RDATAFRAME and RPLOT are one implementation for every runtime**
  (`rapidr_value::datascience`, docs/ide-plan.md decision D7): native
  builds, interpreted programs and the browser run the same arrays, frames
  (with their CSV and JSON readers and writers) and chart model; a runtime
  adds only PRINT and filling a QSTRINGGRID. Charts are drawn by the UI
  kernel, pixel-identical on the desktop and the web and crisp at every
  screen scale (category axes, integer ticks); frames are a columnar
  engine of RapidR's own (a million rows: load 114 ms, sort 182 ms, group
  80 ms, join 60 ms). polars, ndarray and plotters are no longer
  dependencies — 89 fewer crates in the tree, smaller native builds — and
  THIRD_PARTY_NOTICES.md is regenerated. Where the two implementations
  disagreed, one behaviour was chosen: `setcell(row, col, value)` (the
  web took the column first), `randint` includes both ends, `reciprocal`
  of 0 is INF, a missing array reads as empty, `Shape` of an RNUM is
  `(3,)`, `groupby` puts the group column first and orders groups by key,
  `describe` summarises the numeric columns (count, mean, std, min,
  quartiles, max), `transpose` names its first column `column`, charts
  default to 640 × 480 px without a grid, and computed numbers print as
  RapidR prints numbers (`0.3`, not `0.30000000000000004`).
  `examples/web_datascience.rr` and the web IDE's hover help follow.
- **RPLOT's charts look the same, and better, on the desktop and the
  web**: the axes' ranges, bar widths and legend place come from the shared
  model (`Plot::ranges`, `nice_ticks`, `legend_corner`) — round 1-2-5
  ticks without `.0` on whole numbers, a light grid, 2-pixel lines, real
  dashed lines (`--`), bars wholly inside the axes standing on 0, a line
  chart fitting its data, the legend framed in a corner free of data (the
  y axis reaching higher when none is, as Matplotlib's headroom), pies from twelve o'clock with labels outside and white between
  slices. Desktop charts are drawn twice as fine and averaged down (smooth
  edges), with text at the web's sizes.
- **Charts are sharp on high-DPI screens**: `Image.LoadFromPlot` keeps
  what draws the chart again at the screen's scale (`Bitmap::set_redraw`,
  as an SVG picture is), and the web's chart canvas draws at the page's
  scale; the pixels a program reads stay the 1× ones.
- `ToGrid` makes the header the grid's fixed row and no column fixed, so
  every column of the frame shows as data.
- **Web: Open and Save use the user's real files**, through the browser's
  own pickers, as the desktop uses the system's dialogs; the in-page "Save
  As" list of the page's files with its Upload… button is gone.
  QOPENDIALOG / QFILEDIALOG open the system's Open dialog
  (`showOpenFilePicker` in Chrome / Edge, a file input in Firefox / Safari)
  and read the files picked before `Execute` returns; QSAVEDIALOG opens the
  system's Save dialog in Chrome / Edge (`showSaveFilePicker`, FileName and
  DefaultExt proposed). The program reads and writes them with its ordinary
  file I/O: a name a dialog answered is the real file's (its writes go to
  it), any other name stays in the browser's store. Firefox and Safari have
  no save dialog: RapidR asks for the name in a box of its own, and the
  program's writes to it are a download. `Filter` is the pickers' file
  types (the FilterIndex one first; "All files" only when the Filter has
  it — and a file input, which takes one group only, lets every file
  through then, so none is greyed out); Cancel returns 0. Execute with no
  user gesture left (from a timer) shows a small box whose button opens the
  picker. In the web IDE the IDE shows the pickers for the program's
  sandboxed frame and writes back only to the files picked during that run.
  docs/manual/web.md says what each browser does.
- **Web IDE: the run window** follows the IDE's theme (no more Windows-blue
  title bar around the program's own windows, which draw their frames in
  the program's theme), and is big enough for the program's windows and
  dialogs, not only the startup form's design size.
- INPUT's box and the host's prompts select their proposed text, so typing
  replaces it, as Windows' boxes do.
- README rewritten for newcomers (install from the releases first, a quick
  start, what's in it, the platforms checked); COMPILER_MANUAL.md is now
  the contributor manual for today's architecture, and its outdated PDF is
  gone.
- The git history no longer holds the earlier versions of QDockForm and
  QDirListView (close ports of user-contributed RapidQ code) or of the four
  conformance cases and the `tab_control` fixture that reused the manual's
  examples: every commit carries the clean versions instead (same commits,
  authors and messages; today's tree unchanged). Clones made before
  2026-10-06 should be cloned again.

### Fixed
- **RapidQ's examples at run time** (`tools/corpus_run.py`, report in
  docs/corpus-runtime.md): every example that compiles is run on the
  interpreter and as a native build, GUI ones captured beside RC.EXE's
  windows, and what went wrong fixed as RapidQ does it (each checked with
  RC.EXE): `SLEEP .1`; a program's own `TYPE QToolBar`; `Font.AddStyles` /
  `DelStyles` on components and on a QBITMAP's font (`= n` too); a form's
  first Show fires OnResize, OnShow, OnResize; QTIMER on Windows' ticks;
  RLE4 / RLE8 / 16-bit BMPs; aligned controls placed in RapidQ's order at
  the first Show and inside a panel's bevels; status panels 50 wide; a
  QBUTTON's glyph; `ImageList.Handle =`; canvases made in OnShow paint;
  QSTRINGGRID's FixedColor and owner drawing; a QFILESTREAM that can't open
  its file stops the program (`Cannot open file x.`); a borderless form
  never shows scroll bars; QFILELISTBOX's order and `[.]`; hex numbers with
  `?` / `@` digits (`&HFFFF0000???` in RapidQ's CommCtrl.inc), `&HH1`, and
  their low 32 bits past 8 digits; QFONTDIALOG's colours (clWindowText at
  first, system colours kept). RapidQ IDE's `.rqw` window programs are part
  of the corpus now.
- **The font dialog names a system colour by its colour**: given clWindowText (a new
  QFONTDIALOG's Color, and every font that hasn't set one) its colour list showed "Custom"; it
  shows "Black" now (clWindow "White"; one that isn't among the 16, such as clBtnFace, stays
  Custom), the sample drawn in the theme's text colour, and OK keeps Color clWindowText unless
  another colour is picked — so a label given the font back still follows the theme. (RapidQ's
  dialog shows Custom: a deliberate difference.)
- **`QFONTDIALOG.SetFont(Label.Font)` / `GetFont(Label.Font)`** change and
  read the component's own font, and `Label2.Font = Label.Font` copies it
  (an addition: RapidQ's compiler refuses a component's Font there).
- **Native builds read TRUE / FALSE as the program defines them**: RAPIDQ.INC's `CONST True = 1`
  was ignored by native builds (TRUE stayed -1) while the interpreter and the web took it, so
  `IF Port.Connected = TRUE` failed natively in RapidQ's own ComPort example
  (`const_true_redefined`).
- **Text in RapidQ's default font was cramped, letters running together**
  ("program", "start", "Bread", "Price", "Right-click" in labels, edits,
  grids and status bars; worst on a Retina or 150 % screen and on the web).
  RapidR Sans, the font RapidR draws MS Sans Serif with, had Liberation's
  letters made larger twice over (12 % instead of 6 %) and squeezed into MS
  Sans Serif's bitmap widths (an r is 3 pixels there): "Br", "pr", "ar"
  overlapped by up to a pixel, and 816 of 2,028 letter pairs were closer
  than half a pixel at 8 pt. Now every letter is Liberation's own shape, all
  one size (95 %, never narrowed, no letter smaller than the next), with at
  least 0.8 of a pixel between letters: 47 pairs are closer than half a
  pixel (Liberation Sans itself, Arial's spacing, has 85) — the cross-bars
  of t and f, the points of A, V, w. Readability comes first: r, x, y, j,
  C, the brackets, & and % are a pixel wider than RapidQ's bitmap, so text
  measures a little wider than in RapidQ — "program" 40 pixels (RapidQ 38),
  "start" 21 (20), "Password:" 50 (49), "Right-click" and "Notepad -
  untitled" unchanged, a long sentence about 1–2 % (TextWidth and AutoSize
  report what is drawn); tab stops stay RapidQ's. New gallery cases `hello`,
  `spacing` and `words` with RC.EXE's captures beside them, and
  `tools/visual/words.py` (word by word, old | new | RC.EXE, 1×, 1.5×, 2×).
- **Bold Arial ran together too** (a web window's title, "Pantry"; a
  chart's bold title): its bold is the regular letter made heavier, about a
  pixel wider at 12 px, which ate the space between letters. Each bold
  character now takes half a pixel more at 12 px, as Windows' Arial Bold
  is wider than its regular (TextWidth too; RC.EXE: Arial 9 bold "Pantry"
  37 pixels, RapidR 38, before 36).

- **Every component now starts with RapidQ's values, on every runtime.**
  Reading a property right after creating a component gave nothing for 252
  properties that RapidQ gives a value — a button's Cursor, Kind,
  ModalResult and Spacing, a label's Enabled, a form's KeyPreview, Align
  and ShowHint almost everywhere. They now read what RapidQ's own compiler
  reads, the same in native programs, the interpreter and the web. Checked
  against RapidQ, a few values changed: a new form's Left and Top are 0 (it
  opens at the top left of the screen, as in RapidQ, where RapidR used 100),
  CopyMode reads cmSrcCopy, a QDXSCREEN's AllowStretch is off, and
  true/false properties of lists, trees, list views and scroll boxes read 1
  rather than -1 (so `= True` works with RAPIDQ.INC). A button, gauge,
  scroll box or tab control on a form now reads its form's Color, as in
  RapidQ.
- **Memory and file streams read exactly as RapidQ.** Checked against
  RapidQ's own compiler:
  - `ReadStr(n)` always gives n characters, spaces where the stream has no
    more bytes; a QFILESTREAM's `ReadStr(n)` and `Read(S$)` give one more
    character, a space, as RapidQ's do (`ReadBinStr` doesn't).
  - `ReadLine` removes only the CR right before the LF (others stay); a NUL
    in the line ends its text and moves `Position` to the end, as in
    RapidQ. `LineCount` counts the LFs (a last line without one isn't
    counted).
  - `Position` can be set past the end (and, on a memory stream, before
    the start); reads there get no bytes and leave it where it is; a write
    past the end fills the gap with zeros, one before the start writes
    nothing; a `Size` that leaves `Position` past the new end moves it to
    the old end.
  - `ReadAll` (RapidR's) after the stream's start returned nothing, and
    crashed debug native builds; it now gives the rest of the stream. The
    same fix makes `LoadFromStream` (bitmaps, grids, image lists) read the
    rest of a stream that was already read from.

- The web runtime's `Application.Theme` read "modern" after `auto`
  (now `rapidr`) chose the dark or high contrast look; it reads the look
  drawn.
- A docked group's tabs were measured in the regular face but drawn bold:
  the shown tab's title was cut short ("Out…").
- **A window shown from a modal form ignored the user.** Under
  `Form.ShowModal`, `Form2.Show` opened a window that took no clicks or
  keys (and gave the focus straight back). As in Windows, a modal form
  now blocks only the windows that were open when it went up.
- On Linux's Wayland sessions `Form.Hide` left the window on screen (it
  now goes, and `Show` brings it back).
- Keys still held when a window got the focus were typed into it (on
  Windows, the P of a Ctrl+Shift+P that opened a window).
- `RDOCKMANAGER.PaneTitle` of a document that wasn't the active MDI window
  renamed the active one.
- **Debugger: a SUB's own variables showed up in Globals under made-up
  names.** A SUB's STATIC variables and the variables a SUB uses before
  the main program does (RapidQ keeps those between calls) were listed in
  the Globals scope as `S__p` or `SUB S::hits` (with a `#init` twin). They
  are now in that SUB's Locals under the names the program gives them, and
  Globals lists only the program's real globals. A watch or the debug
  console can read and set a SUB's STATIC variables too. The same in VS
  Code (`rapidr dap`) and RapidR Studio.
- **VS Code: an error right after a dot (`Form.` at a line's end) was
  underlined over nothing.** It now underlines the dot.
- **VS Code: Format Document re-indented the second line of a string
  continued with `_` (under `$ESCAPECHARS`)**, which changed the string.
  A string that spans lines is now left exactly as written.

- **Web IDE: an example or opened file showed nothing.** Picking an
  example (Menus, Hello form, …) said "loaded", but no tab opened, the
  Project panel stayed empty and the designer was blank. The examples are
  written with RapidQ's component names (`CREATE Form AS QFORM`, `QMEMO`,
  `QMAINMENU`), and the IDE's reader only knew a form as `RFORM`: it found
  no form, so it dropped every component and the code with them. Now a
  RapidQ name is the RapidR component it stands for, as the compiler reads
  it (the language registry's entries in `lang-data.js` now say which
  component they are), and the designer shows the form: its components,
  the main menu's own menus, a status bar's text, components docked with
  `Align` (and a status bar at the bottom), several properties on one line
  (`Caption = "&New": OnClick = NewText`). A program with no form (the
  console examples) opens in a code module. Opening a local `.rr` or `.bas`
  file does the same, and it runs as it is written, as an example does.
  The run window was already drawing programs at 1:1 (checked at pixel
  ratio 1 and 2, the same pixels as the standalone web runtime); a test now
  keeps it so (`tests/web_ide_examples.mjs`).
- **Web: typing in a Save As dialog went into the program's window
  below.** In the web IDE, Notepad's File > Save As showed an in-page
  dialog; a click in its file name field lost the focus at once, and the
  keys went into Notepad's editor. The dialog was a page element, not a
  window of the UI kernel, and the Open / Save wait put the program's own
  form on the modal list as the innermost modal window: each repaint of it
  (the caret's blink) synced its accessibility mirror, which moved the
  page's focus back to its editor. The in-page dialog is gone (see Changed),
  and the form now goes on the modal list *before* the host is asked, so a
  box the host shows for the dialog is the innermost window and keeps the
  focus, the keys and the clicks. The modal rule is checked with real input
  on the web for every dialog — MESSAGEBOX, MESSAGEDLG, SHOWMESSAGE,
  INPUT's box, QCOLORDIALOG, QFONTDIALOG, a ShowModal form and the Open /
  Save boxes: a click on the window below is refused and the focus stays
  while it repaints, keys never reach it, Tab / Shift+Tab cycle inside the
  dialog, Enter presses its default button, Escape cancels, and the focus
  goes back to the control that had it (`tests/web_modal_focus.mjs`,
  `tests/web_file_dialogs.mjs`).
- **Desktop (macOS): a click on a window under a modal dialog brought that
  window over the dialog.** Its input was refused, but the window took the
  keyboard and covered the box (MESSAGEBOX, colour, font, a ShowModal
  form), native and interpreted alike: the host gave the modal window the
  focus back inside the focus event, and the click's own activation won.
  It now gives it back again once the click is over, so the dialog stays in
  front with the keyboard, as Windows keeps a modal dialog over its owner
  (checked with real clicks and keys on both builds).
- QSTRINGGRID with `FixedCols = 0` (or `FixedRows = 0`) showed from column
  (row) 1: the first scrollable column stays the first one shown, as in
  Delphi's grid.
- RDATAFRAME's `Cell`, `At` and `SetCell` gave strings in quotes on the
  desktop (`"Alice"`); now as the web gives them.
- RJSON on the web: `LoadFile` / `SaveFile` work (the page's files, as
  OPEN's), values `Set` as numbers / booleans as on the desktop; on the
  desktop an object's keys keep their order (as the browser's), `Remove`
  too.
- RDATAFRAME's `LoadFromCsv` on the web reads a file the program saved
  (`EXTRACTRESOURCE`, OPEN), not only the project's.
- **The main program's end is the program's**, as in RapidQ (RC.EXE,
  checked in the Windows VM): a program whose main code ends after
  `Form.Show` (no ShowModal, no DOEVENTS loop) runs OnShow inside Show and
  then ends — its form goes, its timers never tick. The interpreter kept
  such a program's windows open with their timers stopped (so a QTIMER
  never ticked), the web kept them open and ticking; native builds already
  ended. The loop after the main program is gone from the interpreter, the
  debugger and both web builds (`rp_run_app`, `Wait::App`). Conformance
  case `main_ends_after_show`, `tests/web_main_end.mjs`.
- **`Form.` with no member right after the dot** is RC.EXE's `Member  not
  part of class FORM` (the member it read is empty; `Form.Font.` is
  `Member FONT. not part of class FORM`): the next line was joined to it,
  so `Form.` then `Nope x` compiled. A space after the dot (`Form. Caption`)
  and WITH's `.` alone are the same error, as in RapidQ; `Form._` with
  the member on the next line still joins. Cases `member_dot_alone`,
  `member_dot_continued`.
- **`$TYPECHECK ON` checks the names a program reads**, not only those it
  stores into: an undeclared name passed to a method (`List.AddItems
  itme`), a SUB or FUNCTION, printed, or used in an expression or a
  condition is RC.EXE's `Undefined symbol ITME`. A name stored into while
  the check was off is a variable from there on, as in RapidQ; True /
  False and RapidR's own constants stay known (RapidR's additions). Cases
  `typecheck_reads`, `typecheck_reads_ok`.
- **A SUB or FUNCTION written without its parameters has the ones it was
  DECLAREd with** (`DECLARE FUNCTION G (a AS INTEGER) AS INTEGER` then
  `FUNCTION G` reads `a`; RC.EXE's G(4) is 8): `a` was an implicit
  variable there (0). RapidQ's `forms/MinToTaskbar.bas` is written so.
- **A program sees only its own command line**, however it runs. `rapidr
  run-bc prog.rrbc a b` handed the program the runner's own arguments
  (`COMMAND$` was "run-bc prog.rrbc a b"; the IDE opened "run-bc …" as a
  file), and so did a standalone runner's `--bytecode <file>`; now they
  set the program to the file with the arguments after it, as `rapidr run`,
  `rapidr open`, the IDE and built executables do. An old macOS's `-psn_…`
  argument (Finder's process number) is no argument either.
- **`COMMAND$(n)` and `CommandCount`**, as RapidQ's (RC.EXE, checked in
  the Windows VM): `CommandCount` was an unknown name (0) and `COMMAND$(n)`
  ignored `n`. Now `COMMAND$(0)` is the program's file, `COMMAND$(1)` …
  its arguments (`"b c"` one), any other index ""; the bare `COMMAND$`
  stays RapidR's (the arguments joined with spaces). On the web the
  arguments are the page's query string's parts (`?a&b%20c`: `a`, `b c`),
  decoded, or the ones a page hands its program (`window.RAPIDR_ARGS`):
  the web IDE's preview gives none, never its frame's own query string
  (a program there saw "role=run&v=…"). Shared by every runtime (`rapidr_value::command_line`);
  conformance cases `command_line_args` (with the runner's new
  `<case>.args`) and `command_line_none`, the web bundle's query string in
  `tests/web_bundle_console.mjs`.
- `RDataFrame.ToString` / `Print` on the desktop showed only
  `shape: (2, 2)` and a note about polars' `fmt` feature, and `Print`
  printed twice: a frame now prints once, as a plain-text table that is the
  same on every runtime (a header, a rule, the rows — numbers
  right-aligned, `null` for a missing value, the first and last ten rows of
  a longer frame — and `[2 rows x 3 columns]`); `ToString` returns it
  without printing.
- `RDataFrame.Cell` / `CellByName` returned text cells in double quotes
  (`"bob"`) on the desktop: they return the plain text.
- `RNum.Shape` printed "RNum.shape() not implemented" instead of the
  array's shape; a property read the way methods are (`PRINT a.Shape`)
  now reads the property on every data-science component.
- On the web, `RDataFrame.SaveToCSV` / `SaveToJSON` before any other file
  I/O wrote nowhere: data frames use the page's files from the start.
- QSTRINGGRID with `FixedCols = 0` (or `FixedRows = 0`) showed from column
  (row) 1: the first scrollable column stays the first one shown, as in
  Delphi's grid.
- RJSON on the web: `LoadFile` / `SaveFile` work (the page's files, as
  OPEN's), values `Set` as numbers / booleans as on the desktop; on the
  desktop an object's keys keep their order (as the browser's), `Remove`
  too.
- **A FUNCTION's own name inside it** follows RapidQ's compiler (RC.EXE):
  without parameters it is a call of itself (`G = G + 1` recurses);
  with parameters, reading it without arguments is the compile error
  `Expected ( but got "+"` on every runtime. The interpreter read the
  result variable instead, and native builds failed to compile such a
  program. `RESULT` reads the result. Conformance cases
  `function_self_name`, `function_self_read` (checked against RC.EXE).
- `ANNOUNCEMENT.md` announced "1.0.0" and called RapidR a reimplementation:
  it now announces 2.117.0 as the release notes do — RapidR is compatible
  with RapidQ, an original implementation written from the ground up in
  pure Rust.

## [2.117.0] — 2026-10-06

### Added
- **RapidR's brand** (`design/brand/`): an original logo (an R whose
  counter is a play triangle, on a tile), the app icons for RapidR and the
  RapidR Runtime and the file icons for `.rr`, `.bas` and `.rrbc` on macOS
  (`.icns`), Windows (`.ico`), Linux (hicolor) and the web (favicons), the
  README banner and the GitHub social preview — original vector art, MIT,
  wired into the installers and the web IDE.
- **For testers and supporters:** GitHub issue forms (bug report, RapidQ
  compatibility, test report; security reports go privately through
  SECURITY.md), and "Support RapidR" (Buy Me a Coffee) in the README and
  `.github/FUNDING.yml`.
- **The IDE plan** (`docs/ide-plan.md`, ROADMAP Phase 3 as stages I0–I9):
  RapidR Studio on the UI kernel, desktop and web — the designer,
  IntelliSense, Delphi-style linked data and data-science components, a
  debugger, AI through an MCP server in the IDE; every IDE block a public
  component (`docs/ide-components.md`, `docs/ide-ai.md`,
  `docs/q-and-r-components.md`).
- **Every program RapidR builds carries its open-source notices.** A
  `THIRD-PARTY-NOTICES.txt` is written beside every native and interpreted
  executable and into every web build (the zip's root for `bundle-bc` and the
  web IDE's **Build**, `<stem>_web/` for `build --web`; `index.html` links it
  with `<link rel="license">`). It lists RapidR's runtime and every component
  compiled in — the crates of the build's real dependency graph for that
  target, Rust's standard library, the Liberation fonts, SQLite and the C
  code crates bundle, the system's and toolchain's pieces — with every
  licence text in full, so shipping the program with that file is all the
  licences ask. `rapidr notices [<os>-<arch>|web|tools-<os>]` prints one; an
  install ships them (`lib/rapidr/notices/`), so this works offline without
  Rust. Programs see no difference (no new command-line switch).
- **LEGAL.md** (what you may do with RapidR and the programs you build, what
  to ship, trademarks and no affiliation with RapidQ's author or any vendor,
  no warranty) and **docs/licensing.md** (what each output contains, licence
  by licence; what each licence asks; codecs and patents; open questions for
  a professional review). Linked from the README, the web IDE's About and
  Help > Legal…, the desktop IDE's About and `rapidr about`; the installers
  include it (the Windows installer shows it).
- `tools/regress.sh legal`: `cargo deny check licenses`, the repository's
  notices, and `tools/check_notices.py` (builds a program each way and checks
  its notices list every crate `cargo tree` finds).
- **The RapidR Runtime and release packaging** (docs/release-packaging.md,
  `tools/release/`): an installed RapidR finds its files by one rule
  (`RAPIDR_HOME`, `<exe>/../lib/rapidr` with its `release.toml`, else the
  source checkout). Programs run, the IDE works and interpreted executables
  build with no Rust installed (shipped runners, `--target <os>-<arch>`);
  native builds compile offline against the shipped runtime sources, and
  `rapidr setup` installs Rust through rustup only after asking. New
  commands `rapidr run`, `open`, `info`, `ide`; `rapidrw`, the windowed
  launcher. Bytecode format 3 records the oldest runtime a program needs and
  its app type — an older runtime says "this program needs RapidR Runtime
  ≥ x.y" (format-2 files still run). `#!/usr/bin/env rapidr` scripts on
  macOS and Linux. File types per user: `.rrbc` runs on a double click,
  `.rr` / `.bas` open in the IDE with a Run action; a downloaded file
  (quarantine / Mark of the Web) asks once before it runs. Windows GUI
  programs build as windowed executables (no console window). Local build
  scripts for the installers: macOS `.dmg` (the IDE + SDK, and the Runtime
  alone), Windows (Inno Setup), Linux `.tar.gz` / `.deb`, the web bundle,
  SHA256SUMS and an SBOM.
- **The system tray (QNOTIFYICONDATA) on every platform.** RapidQ
  programs put an icon in Windows' notification area with QNOTIFYICONDATA
  and `Shell_NotifyIcon` (shell32) and hear its clicks in their form's
  WndProc. RapidR keeps that one call: the icon shows in the menu bar on
  macOS, the notification area on Windows, StatusNotifierItem on Linux (no
  system package needed) and a small strip at the page's bottom right on the
  web; its clicks reach the form's `WndProc (hWnd, uMsg, wParam, lParam)` as
  Windows' mouse messages. `Application.Icon` reads as a number (RapidQ's
  icon handle), so `NI.hIcon = Application.Icon` shows the program's icon.
- **QDIRLISTVIEW and QDOCKFORM built in**: RapidR's own versions of RapidQ's
  QDirListView.inc and RAPIDQ2.INC's dockable form (theirs call Windows),
  written in BASIC on RapidR's components, so they behave the same on every
  runtime. QDIRLISTVIEW lists a folder (Name, Size, Type, Date Modified),
  goes into folders and up, fires OnFileSelect; QDOCKFORM docks, floats in
  its own window, docks at its alternative place, with the library's title
  and grip styles, a close box, OnDock / OnClose. A program that includes
  QDirListView.inc or RAPIDQ2.INC gets RapidR's.
- **QGLASSFRAME** with RapidQ's compiler's members and values (Transparency
  a byte, Moveable, TransparentColor; 105 × 105): its glass colour over what
  is under it at 100 − Transparency percent (the form's background — RapidR's
  windows aren't see-through), and Moveable: dragging it moves its form.
- **FileRec** (RapidQ's DIR$ companion): FileName, ShortName, Date, Time,
  Size, FileTime of the file DIR$ found last.
- **QCGI**, RapidQ's CGI object (its manual's Appendix A: QCGI.INC 1.6),
  built in on every runtime: the CGI variables as read-only properties
  (Accept, ContentLength, Cookie, QueryString, RemoteAddr, ServerPort,
  UserAgent, …), MaxInput, AutoConvert, Parse and `Get(Name, Value)` —
  pairs read from a GET's QUERY_STRING or a POST's body, `%xx` and `+`
  decoded exactly as the library does (checked against RapidQ itself:
  RC.EXE running the library, `tests/conformance/cases/cgi_*`). A program
  that `$INCLUDE`s `qcgi.inc` gets RapidR's QCGI (the file's TYPE is left
  out, its constants kept; without the file, its constants are built in).
- **QCOMPORT**, a serial port as RapidQ programs have it (RAPIDQ2.INC's
  COMPORT, the manual's QCOMPORT — RC.EXE's own can't be built): Port,
  BaudRate, DataBits, Parity, StopBits, buffers, Connected, Handle,
  BytesNotRead / InQue …; Open, Close, PurgeIn / PurgeOut, WriteString,
  Write, ReadString, Read; OnComError (the library's messages with
  Windows' own text, checked against RC.EXE), OnOpen, OnClose,
  OnWriteString, OnReadString, and OnRxChar when bytes arrive. Ports
  through serial2 on the desktop (nothing extra to install on Linux), Web
  Serial in the browser; tests use scripted ports (`RAPIDR_TEST_COMPORT`),
  never a real device.
- **QDOWNLOAD** (Qdownload.inc): LeechFile fetches Server / Port / File
  into OutVar or OutFile with the library's checks and error messages,
  progress in State or its own StateGauge and SpeedLbl — and the program
  no longer freezes while it waits: its windows paint and its timers tick
  (native, interpreted and in the browser). Port 443 is HTTPS; redirects
  are followed.
- **QMIDI, QWAVE, QCDAUDIO and QVIDEO**, D. Glodt's media objects (RapidQ's
  `QMidi.inc`, `QWave.inc`, `Qcdaudio.inc`, `QVideo.inc`), on one model:
  Open / Close / Play / Stop / Pause, State, FileOpen, Lenght, Error (MCI's
  own texts, checked in the Windows VM), Volume, their Timer and OnChange,
  the libraries' rules exactly; positions by the clock, with or without a
  device. QMIDI reads standard MIDI files with MCI's timing and plays them
  on the system's MIDI output (midir; Web MIDI); QWAVE plays WAV files,
  records (New, Record from the default input — cpal, getUserMedia), Saves
  and Deletes. QCDAUDIO answers as a machine with no disc does. RapidQ's examples `midi.bas`, `wave.bas`, `Cd.bas`, `video.bas`
  compile and start with RapidQ's include folder.
- **QVIDEO plays AVI files** on every runtime with decoders of RapidR's
  own (`objects::avi`): uncompressed DIB (1–32-bit, palettes), Microsoft
  RLE8 / RLE4, Microsoft Video 1 (CRAM), Cinepak and Motion JPEG, and the
  file's PCM sound. Frames follow the clock; the picture is drawn on a
  canvas on Parent's form or in a window of its own (popup / overlapped
  sizes, the picture stretched, seeks landing on key frames — all as MCI
  does in the Windows VM); the browser runs the same decoder in wasm.
  `tools/make_avi_fixtures.py` writes the test clips.
- **QMIDI plays without a system synthesizer** (macOS, Linux, browsers
  without Web MIDI): a built-in General MIDI synthesizer of RapidR's own
  (`objects::synth`) — procedural instruments and drum kit, no SoundFont
  bundled — on the sound device (rodio; Web Audio in the page).
- The media libraries' **Error** keeps MCI's NUL (`LEN` one more than the
  text) and an **Open while a file is open** leaves that file open, as
  RC.EXE runs QMidi.inc / QWave.inc / QVideo.inc; QVIDEO's Caption is MCI's
  128-character buffer.
- A stray **`END STRUCT` / `END TYPE`** is RC.EXE's END (the program ends
  there): RapidQ's `Network/Download/qdownload.bas` compiles.
- A binary operator with nothing after it (`F("a"+, 5)`, `x = 2 +`) is read
  as RapidQ's compiler reads it: dropped, the value is what came before
  (`Qcdaudio.inc` has it four times).
- **The `ENVIRON "name=text"` statement** sets an environment variable
  (it was a silent no-op): split at the first `=` (or, without one, a
  space), names found in any case by `ENVIRON$` as on Windows; in the
  browser a table of the page's own.
- **One web host: the old DOM host is deleted** (docs/web-host-plan.md, "W11"): the UI kernel draws every program's windows on the web as on the desktop, with no `?host=dom`, no `kernel` feature and no `rapidr-rrcss`; the web runtime is 0.4 MB smaller.
  - RWEBNOTIFICATION.Show shows the notification again (a form's Show had taken it), and RROUTER.Route / Hash read the address.
  - QIMAGE.LoadFromPlot loads the chart's pixels, as on the desktop.
  - Application.Icon is the page's icon too.
  - A component bound with DataSource / DataField writes the user's edits to its field.
  - A QIMAGE given a non-BMP file warns and loads nothing, as on the desktop.
  - SLEEP holds a web program whole, as on the desktop. Its timers fire once it waits again, so a timer's handler that opens a box right after a SLEEP gets its answer.
- **Fallback fonts on the web (W7)** (docs/web-host-plan.md, "W7"): text the built-in Liberation fonts can't draw — ✓ and other symbols, Chinese, Japanese, Korean, emoji (in colour) — is drawn with Noto fonts (SIL OFL 1.1) instead of boxes.
  - The fonts ship beside the web runtime, split by Unicode range. A page fetches a chunk the first time its text needs it: labels, edits and window titles alike.
  - Bundles (`bundle-bc`, the IDE's Build) and `rapidr build --web` sites carry the chunks; an installed RapidR never downloads them.
  - `python3 tools/fonts.py` fetches the CJK sources (official Noto releases, pinned by SHA-256) and builds the chunks. Noto Sans and the Symbols fonts are in the repository.
- **The web IDE loads no Google Fonts** any more: its interface uses the system's fonts, and nothing in the IDE, the runtime or a bundle contacts a third party unless the program does.
- **The UI kernel is the web's host by default** (docs/web-host-plan.md, "The kernel host by default, and W6"): the IDE's preview, `bundle-bc` bundles and `rapidr build --web` pages draw the program's forms with the kernel, with no query parameter. `?host=dom` keeps the old DOM host until it is deleted.
  - **Web-only components over the canvas (W6):** RWEBVIEW, RDOM, RWEBAUDIO / RWEBVIDEO and RPLOT are the page's own elements (`#rr-<name>`), placed by the kernel at their components' places, clipped to their parents and hidden with them. Drop-down lists and menus open on a layer above them.
  - **INPUT** with windows shown asks in the kernel's input box.
  - **END** in a native web build closes its windows.
  - **Combo boxes:** typing in one sets ItemIndex as Windows does: -1 while the list is closed, the first matching item while it's dropped down (desktop too).
  - **Grids:** a grid's in-place editor and its drop-down list are in the accessibility tree, and the editor has the focus there (desktop too).
  - **Tests:** every web suite runs on the kernel host (`tests/web_kernel_page.mjs`: mirror elements, real clicks, the canvas's pixels); `tests/web_overlays.mjs` is new.
- **The web on the UI kernel, Stage W4** (docs/web-host-plan.md, "W4 results"): every browser GUI case now runs on the kernel host (`?host=kernel`), 70 of 70, with the desktop's dumps.
  - **Waits:** ShowModal, DOEVENTS, INPUT$ / WAITKEY and dialogs suspend the browser's VM and resume it as the desktop's interpreter serves them (`rapidr_ui_app::waits`), nested ones in order.
  - **Dialogs:** MESSAGEBOX, MESSAGEDLG, SHOWMESSAGE, MSGBOX, the colour and font dialogs are the kernel's own, pixel-identical to the desktop's. Open / Save go through the kernel's file-dialog request, answered by the page's picker.
  - **Timers** run on the shared timer heap, as on the desktop (started by modal waits, held back while a handler waits).
  - **The host:** windows size from every edge and corner; minimized windows line up along the bottom; an edit's `AutoComplete` property becomes its field's autofill hint.
  - **Native web builds** (`rapidr build --web`) build the runtime with the kernel host and wasm SIMD.
  - A grid scrolls a newly selected cell into view before its accessibility tree is made, so the tree's cells are where they're drawn (desktop too).
  - `tools/regress.sh` makes the desktop's GUI captures and compares the kernel host's windows and trees with them at 1× and 2×; `tests/gui_captures.mjs` runs QDOWNLOAD / QMIDI / QWAVE against the tests' own server and scripted devices.
- **The web on the UI kernel, Stage W3** (docs/web-host-plan.md, "W3 results"): with `?host=kernel` in a page's address, or in the web IDE's (it passes it on to its preview), the program's forms are drawn by the same UI kernel as on the desktop. The DOM host stays the default.
  - **Windows on the page** (`rapidr-ui-host-web`'s `WebHost`): the kernel's display list, drawn by the shared CPU renderer at `devicePixelRatio`, with a kernel-drawn frame in the current theme. Windows stack, move by the title bar, size by their edges, and maximize, minimize, restore and close as on a desktop. A scale change redraws them and fires OnScaleChanged; a lost canvas is redrawn.
  - **Input:** the pointer, the wheel, keys, the clipboard events, input methods (the candidate window at the kernel's caret), a phone's keyboard, and autofill.
  - **Accessibility:** an ARIA mirror of the kernel's tree, written in Rust, with the DOM focus on the kernel's.
  - **In the runtime:** the runtime's `Program` / `Windows` for `rapidr-ui-app` and a `WebStore` (`rapidr-runtime-web` feature `kernel`, on in the web runtime's wasm).
  - **Results:** 64 of the 68 browser GUI cases give the desktop's dumps on it. Their windows are byte-identical to the desktop's captures at 1× and 2×, and their accessibility trees match the desktop's: Chrome's tree over the mirror is the kernel's.
  - **Tests:** `RAPIDR_WEB_HOST=kernel` for `tests/web_gui_parity.mjs`, `tests/web_a11y.mjs` and `tests/web_gui_run.mjs`; `RAPIDR_DESKTOP_CAPTURES` compares pixels and trees. `tests/gui_captures.mjs` keeps every GUI case's captures, trees and dumps.
- `rapidr_value::component_defaults`: what both runtimes' registries give a new component, the desktop's own defaults, and the kernel stores' "unset" rule, in one place (the registry's step 1; no property changed).
- **RapidQ's own compiler as the ground truth, at scale**
  (`docs/rapidq-ground-truth.md`): `tools/rc_probe.sh` runs a list of
  programs through RC.EXE in the Windows VM, each in its own folder with its
  `.input` as standard input, and returns RapidQ's exact output;
  `tools/rapidq_truth.py` stages the conformance cases, the console programs
  of RapidQ's examples or a folder of probes, runs them through RC.EXE and
  RapidR (VM, `--native` too) and lists every difference; `--write-expected`
  pins RapidQ's output as a case's `.expected`. Programs that could print or
  touch the registry are never run in the VM. `tests/rapidq_golden/`: RapidQ's
  own output of console examples (`tools/rapidq_truth.py golden`).
- Conformance cases that stop with a run-time error:
  `name.expected-runtime-error` (both backends and the browser), and new
  cases whose `.expected` is RC.EXE's output: `rapidq_print_doubles`,
  `rapidq_int_rounding`, `rapidq_numeric_stores`, `rapidq_operators_int`,
  `rapidq_text_functions`, `rapidq_if_print_else`, `rapidq_booleans`,
  `rapidq_division_by_zero`.
- **QBEVEL and QDIGDISPLAY built in.** The manual documents them as
  components, but in RapidQ they are TYPEs of its include libraries
  (QBevel.inc, QDigDisplay.inc) — the manual's own QBEVEL example doesn't
  include it. RapidR now has both on every runtime, behaving as the
  libraries do: QBEVEL's Shape / Style set its bevels (bsBox, bsFrame) or
  draw two lines at an edge (bsTopLine … bsRightLine); QDIGDISPLAY shows
  Display in 12 × 24 seven-segment cells (characters 32 … 64), in the
  library's colours, sized to it. A program that includes the library
  gets the library's own TYPE, as in RapidQ (QDigDisplay with its own
  bitmaps). Screen readers: a pane named by its caption, an image named
  by its digits.
- **QRECT and QNOTIFYICONDATA as RapidQ's compiler has them** (checked
  against RC.EXE): one shared record model on every runtime — fields are
  32-bit integers cut toward zero (`3.7` → 3), a string stored into one is
  0, out-of-range values -2147483648; QNOTIFYICONDATA's cbSize is 88 and
  read-only, uID starts as the instance handle 4194304, szTip keeps 64
  characters up to a CHR$(0); `SIZEOF` 16 and 24. RapidQ's errors: `Member
  WIDTH not part of class R`, `Component assignment is not yet supported.`,
  `Datatype QRECT not supported in STRUCT`, `N.CBSIZE is a read-only
  value.` (QRECT's Left rounded before, and unknown members read empty).
- **QDXJOYSTICK**, RapidQ's joystick object (missing from its manual; its
  compiler has it): Update, IsLeft / IsRight / IsUp / IsDown, Button(n) —
  plus RapidR's Index, Connected, Name, X / Y / Z / R / U / V (winmm's 0 …
  65535), Buttons, POV and the events OnButtonDown / OnButtonUp / OnMove.
  Gamepads come from gilrs on Windows and macOS, the kernel's evdev on
  Linux (no extra system package to build) and the Gamepad API in the
  browser, all laid out the same way. No joystick reads "not connected",
  never an error. `DECLARE … joyGetPosEx` (winmm) now names QDXJOYSTICK.

### Changed
- **Linux needs OpenSSL 3** (HTTPS uses the system's TLS): RapidR's Linux
  packages run on Ubuntu 22.04 / Debian 12 and newer (`libssl3`); Ubuntu
  20.04 and Debian 11 are no longer supported.
- **Nothing in a program you build is copyleft, cryptographic or
  data-licensed any more** — removed, not just documented (LEGAL.md,
  docs/licensing.md):
  - **HTTPS uses the operating system's TLS everywhere** (`RHttp`,
    `QDownload`): Security.framework on macOS, SChannel on Windows as
    before, and on Linux the system's OpenSSL 3 (`libssl.so.3`, linked
    dynamically, never shipped), with the system's certificates. `ring`,
    `rustls`, `rustls-webpki` and `webpki-roots` (CDLA certificate data) are
    gone: no cryptographic code is compiled into a program. **Linux:**
    programs need the system's `libssl3` (every current distribution has
    it; the `.deb`s depend on it), and building needs `libssl-dev`
    (docs/release-packaging.md).
  - **MP3 is decoded by `nanomp3`** (MIT OR Apache-2.0, pure Rust, a port
    of the public-domain minimp3) instead of rodio's `symphonia` (MPL-2.0);
    WAV, Ogg Vorbis and FLAC stay rodio's. PLAYWAV / PLAYSOUND play the same
    files.
  - **RPLOT's charts draw their text in the built-in Liberation Sans** (with
    `ab_glyph`, through plotters' backend interface) instead of the
    system's fonts through `font-kit`: the same size and placement as
    before, the same on every system. With font-kit went `dwrote` and
    `option-ext` (MPL-2.0), FreeType (whose licence asks for credit in the
    documentation) and the charts' fontconfig link; the CLI no longer uses
    `dirs`.
  - **winit's KDE blur bindings are RapidR's own**: crates.io's
    `wayland-protocols-plasma`, generated from KDE's protocol files (some
    LGPL-2.1-or-later), is replaced in every build — the workspace, every
    generated program, an install's home — by
    `crates/patches/wayland-protocols-plasma` (MIT): the same Rust names,
    from RapidR's own protocol file, which no compositor announces (winit
    finds no blur manager; RapidR never asked for blur).
  - The Linux notices now also carry the notices of what crates' code is
    generated from or built with: every Wayland protocol description
    (MIT, HPND-sell-variant), xcb-proto (X11), the Cantarell font in
    winit's Wayland title bars (OFL-1.1) and, on every target, the Adobe
    Glyph List in `read-fonts` (BSD-3-Clause).
- **The licence guard is strict**: what a program may contain is the
  permissive list only — MIT, Apache-2.0 (and WITH LLVM-exception),
  BSD-2/3-Clause, ISC, Zlib, 0BSD, BSL-1.0, Unlicense, Unicode-3.0, CC0-1.0,
  and OFL-1.1 for fonts. deny.toml allows only that (for the whole
  workspace, tools included) and bans the replaced crates; the notice
  generator refuses any kind of output with anything else (or a replaced
  crate, or a Wayland protocol file under another licence); and
  `tools/check_notices.py` checks every kind's graph against its own copy
  of the list. `tools/regress.sh legal` exits 1 on any failure, and only the
  web stage needs the repository served (`RAPIDR_URL`, else port 8765).
- QDirListView and QDockForm's built-in libraries rewritten as RapidR's own
  code (their bodies followed user-contributed RapidQ libraries too closely);
  same members, events and behaviour. Four conformance cases that reused the
  RapidQ manual's examples replaced by our own programs covering the same.
- Web bundles carry `THIRD-PARTY-NOTICES.txt` instead of `LICENSE-RapidR.txt`,
  `THIRD_PARTY_NOTICES.md` and `LICENSES.md` (it holds RapidR's licence and
  the full texts the others lacked).
- The SDK's cut-down vendored crates (those only other platforms compile)
  keep their licence files.
- **DIR$ as RapidQ's**: attribute 0 lists files only, faDirectory (&H10)
  folders too with `.` and `..` first, names sorted without regard to case,
  wildcards (`*`, `?`) of any case, `\` as a folder separator; dot files
  only with faHidden.
- **Hiding a modal form no longer ends its ShowModal** (VCL's, so RapidQ's):
  only Close or ModalResult do — a program can hide its window into the tray
  and show it again (desktop, interpreter, and the web on both its hosts).
- The desktop host's `Desktop` (the forms' kernel sides, stacking, the modal list, the window command and event queues, the input entry points) and runtime-core's window-command and test-input glue now live in `rapidr-ui-app`, shared by the desktop and web hosts. The desktop is unchanged: every GUI case's captures, accessibility trees and dumps are byte-identical at 1× and 2×.
- The web runtime's wasm is built with wasm SIMD (`tools/build_web_artifacts.sh`, its own `target/wasm-simd`). With the kernel host it grows from 6.27 to 10.18 MB raw (1.95 → 2.70 MB brotli).
- **Operands side by side read as RapidQ's compiler reads them.** RC.EXE
  takes `SetRenderMode(A A OR B)`, `-9(COS(x))`, `x = 16 374739` and the
  like — its operator stack runs on, the value is the operand stack's
  bottom (`A B OR C` is A, `-9(COS(x))` is 9, `(2 3) + 1` is 2) and the
  other operands are still worked out (a FUNCTION among them is called).
  RapidR refused them; both compilers now read them the same way (checked
  line by line against RC.EXE: `tests/conformance/cases/
  juxtaposed_operands.bas`). So `CASE 4, 7<TAB>C& = -2` (RapidQ's
  `reminder/dayfunction.bas`) is the list 4, 7 with no body, as in RapidQ —
  RapidR used to run the assignment.
- **RapidQ-exact numbers and more, checked against RC.EXE** (native,
  interpreted and web alike; details and evidence in
  `docs/rapidq-ground-truth.md`):
  - PRINT shows a fractional number with 9 decimals (`3.500000000`), Delphi's
    digits, and a whole one as a 32-bit integer (beyond: `-2147483648`, as
    for an infinity or NaN); STR$ has 9 significant digits (`0.333333333`,
    `1.23456789E9`, `1E-5`). They were the shortest form / 15 digits.
  - PRINT's (and LPRINT's) comma is the semicolon — no 14-column zones; a
    leading separator is accepted (`PRINT , "x"`).
  - INT and FIX truncate toward zero; ROUND, CINT and CLNG are `INT(x +
    0.5)` (2.5 → 3, -2.5 → -2, -2.7 → -2); ROUND, CINT, CLNG, CEIL and FLOOR
    are 32-bit.
  - A store into an integer variable truncates (2.7 → 2), beyond 32 bits it
    is -2147483648; a BYVAL parameter rounds half to even; a FUNCTION's
    result isn't converted; DWORD is 32-bit signed; SINGLE is a real 32-bit
    float. They rounded half to even, wrapped, converted results, kept DWORD
    unsigned and SINGLE double.
  - `\` rounds its operands as CINT; MOD, AND, OR, XOR, NOT, SHL, SHR take
    32-bit operands rounded half to even (`7.5 MOD 2` = 0); `\` and MOD by
    zero stop the program ("Division by zero"), `/` by zero is an infinity
    (all three gave 0); a NaN compares equal and less; `&H80000000` …
    `&HFFFFFFFF` are negative; INV is -1 without an inverse.
  - VAL skips spaces and reads the number at the start (`"12abc"` = 12);
    HEX$ has 8 digits; BIN$ 32 bits; REPLACE$ appends past the end;
    INSTR never finds an empty needle and counts from a start before the
    text (`INSTR(-5, "abc", "b")` = -4); CHR$ / STRING$ round a real code;
    CONVBASE$ of a negative number gives its 32 bits;
    Clipboard.GetAsText(n) gives n - 1 characters.
  - A component's Boolean property or method result reads 1 when true
    (`IF Check.Checked = True`, RAPIDQ.INC's True = 1); FILEEXISTS and
    DIREXISTS give 1; a QFORM starts Enabled.
  - An undeclared variable a SUB uses before the main program does is the
    SUB's own (kept between calls), as RapidQ's one-pass compiler has it;
    `DIM m` without AS is a DOUBLE; `""` inside a string is no escaped quote
    (two strings side by side — the web IDE's designer writes quotes as
    `CHR$(34)` now); STRING * n is always n characters (padded with spaces,
    n spaces at first; `STRING * 0` empty).
  - A PRINT right before the ELSE of a single-line IF stays on its line;
    `CASE IS = "l" AND x = "d"` compares first, then ANDs; inside a TYPE's
    own code a store into its property field doesn't call the setter.
  - A component's Color before the program sets one reads RapidQ's
    system colours: clBtnFace (`&H8000000F`) for a QFORM / QPANEL, the
    parent's Color for a QLABEL / QCANVAS / QGROUPBOX (clWindow without
    one), clWindow (`&H80000005`) for the others — it read white. A system
    colour is drawn in the theme's colour (clBtnFace: F0F0F0), so a form's
    Pixel reads the F0F0F0 it shows, and TextOut blends over it. A QCANVAS
    shows its parent's colour where nothing is drawn, not its own Color
    (RapidQ's TPaintBox). The desktop's "creation white" rule is gone.
  - Font.Color reads clWindowText (`&H80000008`) until set — a new QFONT's
    Color too — and a parented component its parent's (ParentFont, drawn
    so too); it read 0. Form.Pixel reads -1 before Show, after Close,
    outside the form and over a window of its own (a panel, a button …),
    and a label's / canvas's / image's pixel over those.
  - TIMER is the seconds since local midnight (it was since 1970 on the
    desktop, since the page loaded on the web); TIME$ and DATE$ are local.


### Fixed
- Inside a TYPE extending a component, `DIR$`, `TIMER`, `DATE$` and the
  other builtins written without parentheses, and CALLFUNC, were taken for
  the component's members (a native build's DIR$ loop stopped early).
- In `CREATE x AS Type` (a TYPE), a nested CREATE's own `Parent = …` was
  overwritten, and `Field.Member = …` (`AltPanel.Parent = Form`) wasn't the
  instance's field.
- What a program draws before its form shows (a QCANVAS's picture, a
  QDIGDISPLAY's digits) is made at the screen's scale from the start: a
  form is drawn at the host's scale before its window exists, and a window
  that opens on a screen of another scale says so (OnScaleChanged). Before,
  a high-DPI window could show (and a test capture) an enlarged, smoothed
  1× picture until something drew again.
- The GUI cases `glass_frame`, `dock_form` and `bevel_display` give their
  forms' ClientWidth, so their pixels are read at a real screen's scale
  (they failed with real windows on a 200 % Windows screen).

## [2.116.0] — 2026-10-05

### Added
- **Themes for the UI kernel's look**, beside RapidQ's classic one (still
  the default, byte for byte): `modern` (flat, Windows 11-like: rounded
  controls, an accent colour, focus rings, thin scroll bars), `dark` (the
  same in Windows 11's dark colours) and `highcontrast` (Windows' High
  Contrast Black: 7:1 text, 3-pixel focus rings). `$THEME name` picks one,
  `Application.Theme = "dark"` switches at run time (RapidR's; RapidQ's
  Application has no Theme), `RAPIDR_THEME` sets a default for programs
  that name none, and `auto` follows the system's dark / high-contrast
  setting. A theme never moves or resizes anything, nor changes fonts; the
  program's own colours (`Color`, `Font.Color`) win. The old FLTK theme
  names keep working (classic or modern). Real windows get a light or dark
  title bar to match. The web keeps its own look for now and reads the
  names back.
- GUI test hooks: `RAPIDR_TEST_MESSAGE_DIALOG` (what each message box
  answers, by button caption) and `RAPIDR_TEST_DIALOG_HOLD=ms` (a dialog a
  hook answers stays open, waited for as the user's, before the answer);
  the `dialog_timers` case checks timers during every dialog, native and
  interpreted.
- **Direct3D Retained Mode** (RapidQ's QD3DFRAME, QD3DMESHBUILDER,
  QD3DMESH, QD3DFACE, QD3DLIGHT, QD3DTEXTURE, QD3DVISUAL, QD3DWRAP,
  QD3DVECTOR and QDXSCREEN's 3D methods) on native, interpreted and web
  builds: frames, faces, `.X` models (text and binary — every model of
  RapidQ's examples loads), lights, the camera, textures, wraps, shadows
  and blended transparency, drawn by RapidR's own software rasterizer into
  the screen at Render — lit flat or Gouraud, back faces culled, sharp at
  any display scale.

### Changed
- **QREGISTRY on Windows is Windows' own registry**, as RapidQ's was, in
  native and interpreted builds alike (Microsoft's `windows-registry`
  crate). It answers as the per-user store does — both are worked out in
  one place (`rapidr_value::registry`): the same paths, data types, sizes
  and 1 / 0. Keys a program may only read (HKEY_LOCAL_MACHINE without
  elevation) open and read; changes to them answer 0 and change nothing.
  HKEY_PERFORMANCE_DATA and Windows 9x's HKEY_DYN_DATA aren't keys there.
  `RAPIDR_REGISTRY=<file>` still puts the keys in that file, on Windows
  too, so test runs never touch the machine's registry; macOS, Linux and
  the web keep the per-user store. Checked on Windows 11
  (`tools/windows/registry_check.ps1`: one program interpreted, as an
  interpreted build and as a native build, the same as the store's run,
  what it left checked with `reg query`).
### Fixed
- Interpreted desktop builds run timers' handlers while a dialog waits for
  the user — MESSAGEBOX, MESSAGEDLG, SHOWMESSAGE, MSGBOX, the Open / Save /
  colour / font dialogs, INPUT$, a kernel-drawn pop-up menu — as native
  builds and RapidQ do, instead of in a burst once it closed. Each is a wait
  the interpreter serves between instructions, as ShowModal is, and ends
  with the dialog's answer. A timer's handler may open a dialog of its own
  over another, or close a modal form under a box.
- A timer whose handler waits (a dialog, ShowModal) doesn't fire again
  until that handler returns, and a timer never waits behind another
  handler that waits (two timers due together, a click and a timer): each
  fires once the handler before it has run, or inside its wait —
  interpreted builds now agree with native.
- A dialog shown before the first ShowModal or DOEVENTS starts the
  program's timers, as ShowModal does (RapidQ's timers tick during any
  dialog).
- Linux (Wayland): a window drawn on the GPU no longer stops its program
  while it can't be seen — minimized, on another workspace, the screen
  locked. A FIFO swapchain waited in present for the compositor's frame
  callback, which doesn't come then, and the whole program (its timers
  too) waited with it; on Wayland windows now present in Mailbox mode.
  Found in an Ubuntu VM whose session had locked.
- A computer whose only GPU for vello is a software one draws on the CPU
  even when an OpenGL adapter is also there (an Ubuntu VM: Mesa's software
  Vulkan beside virgl's GL, which vello can't use): only Vulkan, Metal and
  DX12 adapters count.
- QREGISTRY as TRegistry: GetDataType is 0 (unknown) for the registry's
  other kinds of value (REG_MULTI_SZ, REG_QWORD, …; they were binary) and
  the store's file keeps their kind; KeyExists of a root (`""`, `"\"`) is
  always 1; RenameValue puts the value last (TRegistry deletes, then
  writes); MoveKey into the key's own sub-key is refused whatever the case
  of the names.

## [2.115.0] — 2026-10-05

### Added
- RapidR is checked on Windows 11 (ARM) and Linux (Ubuntu 24.04 with
  Wayland) besides macOS: the conformance suite on both backends, the GUI
  events headless and with real windows, the unit tests, and on Windows
  what a screen reader sees (`tools/windows/uia_probe.ps1`; macOS:
  `tools/macos/ax_dump.swift`). The fixes below came from these runs.
- **RapidQ's DirectX 2D objects on every runtime** (native, interpreted,
  web; docs/directx-plan.md): QDXSCREEN draws on an off-screen surface that
  shows on `Flip` (Init, AutoSize, AllowStretch, Fill's DirectDraw colours,
  Pixel, text, Draw / StretchDraw / CopyRect, TextRect, Rotate, View.*;
  OnInitialize / OnInitializeSurface when its form first shows, or once it's
  put on a form already shown; FullScreen covers the screen with the surface
  scaled to fit). QDXIMAGELIST draws DelphiX image libraries (`.DXG`) with
  their transparency and patterns. QDXTIMER keeps FrameRate, fires once a
  frame with Interval 0, and with ActiveOnly only while the program is
  active. Screen text is MS Sans Serif 8, as in RapidQ.
- QDXSOUND (DirectSound) on every runtime: a WAV file played at its
  Frequency, Volume and Pan (DirectSound's decibels), Looped or once;
  Playing, Position and Size; Play, Stop. rodio plays it on the desktop,
  Web Audio in the browser.

### Changed
- The host-neutral half of the desktop's program glue (the kernel's events,
  forms, the modal list, timers, waits, lists, menus, the GUI test script
  and hooks, the dialogs' requests and answers) is a crate of its own,
  `rapidr-ui-app`, behind two traits (`Program`, `Windows`), so the web host
  can share it (docs/web-host-plan.md, W2). Nothing a program sees changed:
  every GUI case's captures, accessibility trees and dumps are byte-for-byte
  the same.
- The test runners delete each native build once its program ran
  (`tests/cargo_builds.mjs`): a full run left 50–100 GB behind.

### Fixed
- macOS: a borderless form (BorderStyle bsNone) no longer can freeze its
  program. Asking whether such a window was maximized gave it a title bar
  for a moment, which resized it, which asked again; it is now never asked
  (the user can't maximize it).
- Native builds: `Obj.Sub.Method(…)` (`DX.View.SetFront(10)`,
  `Printer.Font.DelStyles(3)`) calls the sub-object's method by its
  combined name on the object, as the interpreter does; it went to a value
  read from the object instead.
- Windows: programs built with `rapidr build` are `.exe` files. An
  interpreted build was written without the extension, and a native
  build's copy next to the source was silently skipped.
- Windows on ARM builds without clang: RHTTP's TLS is Windows' own
  (SChannel, kept current by Windows Update); elsewhere it stays rustls.
  `ring`, rustls' crypto, needs clang to build for Windows on ARM.
- Windows: native builds work when RapidR lives under a Windows path (the
  generated Cargo.toml's paths are escaped; `\U…` was read as an escape).
- Windows: QDIRTREE's Directory is a plain path (`C:\Users\…`), not the
  `\\?\C:\…` form `canonicalize` gives — the tree never found its
  directory under its root (`C:\`), so nothing was selected.
- Windows: INPUT$ from a pipe or a file reads its characters instead of
  waiting for the console's keys.
- Screen readers hear labels: a QLABEL's caption (and a status bar's
  text) was given to AccessKit as a name, which it reads from the value
  for labels — Narrator / NVDA (UI Automation) and VoiceOver got empty
  text. Checked on Windows 11 with UI Automation (names, values, toggles,
  ranges, Invoke / Toggle / SetValue running the handlers) and on macOS.
- Computers whose only GPU is a software one (Windows' WARP, Mesa's
  llvmpipe: virtual machines, remote desktops) draw on the CPU: WARP
  crashed in vello's shaders (an access violation in d3d10warp.dll), and
  vello_cpu is faster there anyway. `RAPIDR_RENDERER=gpu` still forces the
  GPU.
- Wayland: a size the window system applies at once (the status bar's
  size grip, a program setting Width / Height) is a resize the program
  hears about (OnResize, Width / Height).

## [2.114.0] — 2026-10-04

### Changed
- **FLTK is gone: the UI kernel is RapidR's only desktop host**, with no
  fallback. `RAPIDR_HOST` and `RAPIDR_THEME` are no longer read
  (`$THEME` stays and draws the classic look); `rapidr build --host kernel`
  only notes it's no longer needed, `--host fltk` is an error. Smaller
  executables (the CLI −2.8 MB, every interpreted program −2.1 MB) and
  CMake is no longer needed to build.
- RSQLITE is SQLite itself on every runtime: the web runs SQLite compiled
  to wasm (built with the desktop's options) instead of a hand-written
  imitation, and opens `.db` assets with SQLite's own reader. A query
  returns rows when the statement has columns (`WITH … SELECT`, `EXPLAIN`,
  `INSERT … RETURNING`); several statements in one Query all run.
  OnConnect, OnQueryDone, OnDisconnect and OnError fire on every runtime;
  QueryScalar works everywhere.

### Added
- SQL parameter binding (SEC-07): `DB.Query(sql, value1, value2, …)`
  binds values to `?` placeholders (arrays are flattened), `AddParam` /
  `ClearParams` queue them — RSQLITE on every runtime, RMYSQL on the
  desktop. Bound values can't change the statement (no SQL injection).
- The web host plan (`docs/web-host-plan.md`) and its spike: the UI kernel
  drawn in the browser on a canvas, byte-identical to the desktop, with an
  ARIA mirror for screen readers; the drawing code shared by both hosts
  (`rapidr-ui-render`).
- `tools/linux/check.sh`: the committed tree built and tested on Ubuntu in
  Docker, headless and on X11 windows — no remote CI needed.

### Fixed
- Timers keep firing, and their changes are drawn, while a native menu is
  held open (context menus and the macOS menu bar), as on Windows; no CPU
  spin while a menu is open.
- A console program never starts the windowing system (it no longer needs
  a display on Linux servers, nor becomes a GUI app on macOS); without a
  display, programs run on without showing windows.
- Screen readers get a window's contents on their first question (macOS).
- macOS: no size grip drawn in the window's rounded corner (dragging there
  still resizes).
- Web: RowSeek was one row off. Desktop: only the first of several
  statements in a Query ran.

## [2.113.0] — 2026-10-04

### Added
- The IDE written in RapidR (`examples/ide.rr`) runs on the UI kernel host:
  RDESIGNSURFACE and RCODEEDITOR are shared models (FLTK, the kernel and the
  web answer from them), the code editor is the kernel's editor with BASIC
  colours and a line-number gutter, and the editor can carry styled runs
  (colour, bold, italic, underline, font) for QRICHEDIT's next step.
- Screen readers on the web: ARIA roles, names, values and states from the
  same accessibility rules as the desktop kernel (lists, trees, grids, tabs,
  menus, status bar panels), a polite live status bar, Tab order by
  TabOrder / TabStop, Alt + `&` letters, Enter → Default, Escape → Cancel.
  `tests/web_a11y.mjs` compares the browser's tree with the kernel's for
  every fixture.
- MESSAGEBOX / MESSAGEDLG show their type icons (warning, error,
  information, question) with Delphi's layout and captions on FLTK, the
  kernel and the web; MESSAGEBOX beeps on the desktop.
- QCOLORDIALOG: Style, `Colors(1 TO 16)` and the custom colour editor.
  QFONTDIALOG: Name / Size / Color, styles and effects, Min / MaxFontSize,
  options, OnApply, GetFont / SetFont.
- QFORM.WindowState (normal, minimized, maximized) on all three runtimes.
- Kernel host: double clicks in the VCL's order (also fixed on FLTK and the
  web), the real editor for tree / list view / grid in-place edits, edit
  after a pause on the selected item, F10 / Alt for the menu bar, drag
  selection by words and paragraphs with auto-scroll, QSTATUSBAR's size
  grip (also on FLTK), the wheel on grids and list views.

### Fixed
- `Obj.Member` without parentheses: the interpreter and the web read a
  property where RapidQ calls a method (`WHILE MySQL.FetchRow` never looped),
  and native builds called a method where it's a property (`UpDown.Max`,
  `ProgressBar.Max`, `ScrollBar.Min / Max`, `ListBox.Columns` read nothing).
  Which one it is now depends on the object's type, the same everywhere.
- Web: Checked reaches check boxes and radio buttons; ReadOnly edits are
  read-only; controls in a disabled panel are disabled; a group box's
  caption set later keeps its components.
- FLTK: OnClick / OnDblClick for panels, labels, group boxes, scroll boxes
  and forms; a designed component's colour from a QCOLORDIALOG; code editor
  colours after non-ASCII text.
- The CPU renderer's synthetic italics lean forward.

### Changed
- MySQL client 28 without derive, TLS or system zlib (drops the
  unmaintained proc-macro-error2).

## [2.112.0] — 2026-10-04

### Added
- The UI kernel host (`RAPIDR_HOST=kernel`) now runs every desktop GUI test
  program identically to FLTK (native and interpreted, 1× and 2×):
  - drawing surfaces — QCANVAS, a form's own drawing, QIMAGE (BMP, $RESOURCE,
    SVG, icons), speed buttons' glyphs — sharp at 2×;
  - text — the kernel's own editor for QEDIT (PasswordChar, Alignment,
    HideSelection, MaxLength, CharCase, word / all selection, undo, the
    Cut / Copy / Paste menu), QMEMO and QRICHEDIT (plain text, word wrap,
    scroll bars), the editable QCOMBOBOX box; the mouse wheel; IME;
  - dialogs — MESSAGEBOX / MESSAGEDLG / SHOWMESSAGE drawn by the kernel,
    the system's Open / Save dialogs (as sheets on macOS), colour and font
    dialogs;
  - the platform — window frames from BorderStyle / BorderIcons, cursors
    (Screen.Cursor, each component's, I-beams, resize arrows), the mouse on
    the screen, the work area.

### Fixed
- Desktop: QCOLORDIALOG's Color is RapidQ's integer (`&HBBGGRR`), not a
  `"#RRGGBB"` string.
- "MS Sans Serif" / "Microsoft Sans Serif" are drawn and measured with the
  sans face.

## [2.111.0] — 2026-10-04

### Added
- The UI kernel host (`RAPIDR_HOST=kernel`) draws and drives most of
  RapidQ's components, the same as on FLTK (native and interpreted, 1× and
  2×): panels, group boxes, scroll boxes and a form's scroll bars, splitters,
  status bars, MDI child windows; check boxes, radio buttons, cool / oval
  buttons, gauges and progress bars, up-downs; the in-window main menu and
  pop-up menus (the macOS menu bar natively); list and combo boxes (owner
  drawn too), list views, string grids (OnDrawCell, editing, drop-down
  columns), headers, tree views and outlines, directory trees and file
  lists. 43 of 54 GUI test programs now run identically on both hosts.

### Fixed
- Kernel host: a Color the program sets is painted, even white.

## [2.110.0] — 2026-10-04

### Added
- Desktop host Stage 3: `crates/rapidr-ui-host-winit` — RapidR's own
  desktop host, next to FLTK (`RAPIDR_HOST=kernel`; FLTK stays the
  default): winit driven by the program's own loop, vello on the GPU or
  vello_cpu on the CPU (`RAPIDR_RENDERER=cpu|gpu`, falling back to the CPU
  by itself), a screen-reader tree per window (AccessKit), a headless host
  for tests. ShowModal (nested, native and interpreted), DoEvents, timers,
  OnShow / OnLoad / first OnPaint / OnClose / OnResize, the test hooks, and
  `RAPIDR_TEST_A11Y` (each form's accessibility tree as JSON).
  `rapidr build --host kernel` builds a native program with both hosts.
  16 GUI test programs already run on it, natively and interpreted, with
  the same results as on FLTK (`tools/regress.sh` runs the hosts matrix).

## [2.109.0] — 2026-10-04

### Added
- Desktop host Stage 1: the runtime reaches the desktop GUI only through
  `rapidr-runtime-core`'s `ui` facade (63 functions + the platform's), with
  a host switch (`RAPIDR_HOST=fltk|kernel` once both are built), a safety
  net that queues program handlers fired inside a window callback, and the
  test hooks' parsing shared by both hosts — FLTK's behaviour unchanged.
- Desktop host Stage 2: `crates/rapidr-ui-kernel`, the GUI-free UI kernel
  (forms built from the component store, focus with TabOrder / TabStop,
  input routing to events in RapidQ's order, display lists, parley text at
  RapidQ's sizes without kerning, label / button / track bar / tab control /
  single-line edit with IME, an accessibility tree); it builds for the web
  too. Shared drawing ops (`rapidr_value::objects::ops`) and accessibility
  nodes (`objects::a11y`) for every runtime.

## [2.108.0] — 2026-10-04

### Changed
- wasm-bindgen 0.2.129 (js-sys / web-sys 0.3.106) across the workspace and
  in generated web projects — install the matching CLI with
  `cargo install -f wasm-bindgen-cli --version 0.2.129`. It's what the new
  desktop host's wgpu needs, so both can share one workspace.
- `crates/rapidr-ui-proto`: the desktop host's Stage 0 — a program's loop
  driving winit (`pump_app_events`) with nested modals, timers and live
  resize; a CPU renderer (vello_cpu) and a headless host for tests.

### Fixed
- Web: a component's FontSize is points, as on the desktop and in RapidQ
  (12 is 16 pixels; it was 12 pixels). Font sizes become whole pixels the
  way Windows rounds them (10 pt is 13 pixels) everywhere text is drawn or
  measured — TextWidth / TextHeight measured at 13.33 pixels while text was
  drawn at 13.

## [2.107.0] — 2026-10-04

### Added
- `docs/desktop-host-plan.md`: the staged plan for moving the desktop from
  FLTK to RapidR's own UI kernel (winit + vello + parley + AccessKit).
- `Anchors` and `Constraints` on every visual component (RapidR's, as in
  Delphi; RapidQ had Align only): `Anchors = akLeft + akRight` keeps a
  control's distances to its parent's edges as the parent resizes
  (stretching with both, keeping its centre with neither); `MinWidth` /
  `MinHeight` / `MaxWidth` / `MaxHeight` (also as `Constraints.MinWidth`)
  bound every size it takes — set by the program, by Align, by Anchors, or
  by the user (a form can't be dragged smaller). One model in
  `rapidr_value::layout`, the same on every runtime; `akLeft` … `akBottom`
  need no include, and a program's own names win.

### Fixed
- Web: a text box narrower than 8 pixels keeps the width the program gives
  it.

## [2.106.0] — 2026-10-04

### Added
- Web: the VM runs in time slices (~10 ms): a busy loop — `cpuhog: GOTO
  cpuhog`, a game loop without DoEvents, a long computation — no longer
  freezes the page, what it draws and prints shows as it runs, and the
  IDE's Stop always works; events still run only when the program waits
  (DoEvents, ShowModal, a dialog, the end of its main code), as on the
  desktop. DoEvents and an empty INKEY$ no longer cost 4 ms each. The
  desktop interpreter is unchanged (the check is compiled out there).
- `tools/regress.sh` runs the desktop GUI events at 2× too (`RAPIDR_SCALE=2`).
- `Screen.Scale` and `Form.Scale` (RapidR's): device pixels per pixel (2 on
  a Retina screen, 1.5 at 150 %) — programs keep RapidQ's pixels, this
  tells them how fine the screen is; `Screen.PixelsPerInch` reads RapidQ's
  96. `OnScaleChanged` (RapidR's): a form moved to a screen with another
  scale is told, then drawn again (OnPaint).

### Fixed
- A QFORM's Visible is RapidQ's: False until the form shows, then whether
  it shows (it read True from the start); `Form.Visible = True` shows the
  form with its OnShow on the desktop too (it did nothing for a form not
  shown yet) — inside the form's own CREATE as well, where the form shows
  once the program waits (Splitter.bas' "required" `Visible = 1`), as in
  the browser.

## [2.105.0] — 2026-10-03

### Added
- `crates/rapidr-ui-proto`: a prototype of RapidR's own desktop host on
  winit + wgpu/vello + parley + AccessKit, with muda / rfd / arboard for the
  menu bar, dialogs and clipboard — a QFORM with QLABEL, QBUTTON, QEDIT,
  QTRACKBAR and QTABCONTROL drawn from the shared models at the screen's
  scale, keyboard focus, a screen-reader tree, an offscreen capture. Its own
  lockfile, outside the workspace (wgpu needs a newer wasm-bindgen than the
  web build's); every dependency permissive (MIT / Apache-2.0 / BSD / Zlib).
- `tests/corpus_web_compare.mjs`: RapidQ's portable example programs run in
  the browser (the web IDE's compiler and VM) and on the desktop
  interpreter side by side; what they print and every form's and
  component's properties, as the program reads them, are compared. The
  browser gets the desktop's screen size, and a program that never yields
  to the page is reported, not waited for.
- `RAPIDR_MENU=window`: a QMAINMENU is a bar inside its form on macOS too,
  as on Windows, Linux and in the browser (the same ClientHeight
  everywhere); without it, macOS keeps the system menu bar.

- A component type's name used as a value is the newest object of that
  type, as in RapidQ: `Parent = QFORM` in a toolbar or main-menu include puts
  it on the form created last (RapidQ's ToolTest example), and in a TYPE's
  constructor `Parent = QFORM` is the newest form.

### Fixed
Found by running the portable corpus in the browser against the desktop:
- Web: OnShow fires — on each ShowModal and when Show shows a hidden form,
  as on the desktop (it never fired in the browser); the desktop now fires
  it too when a form is shown again after Hide / Close.
- Desktop: SHOWMESSAGE is RapidQ's message box with an OK button, titled
  with Application.Title (it only printed the message); under a test's
  hooks it still prints and goes on.
- Web: `$RESOURCE` files are found in any case (`BACK1.BMP` for back1.bmp),
  as on Windows.
- Web: only the forms a program shows appear (Show, ShowModal,
  Visible = True), as on the desktop and in RapidQ, where a form starts
  hidden; a dialog form created at start no longer shows over the main one.
- Web: FILEEXISTS finds the program's files (the project's assets and the
  files it saved), and files are read from the project's assets, with
  Windows' rules (any case, `\` or `/`) — a program loading its
  bitmaps or data files works in the browser (the VECTOR font demo, the
  file splitter's tiled background).
- Web: a component without a Parent isn't shown (it went into the first
  form), as on the desktop.
- Web: a button, edit or combo box is exactly the size the program gives it:
  the theme's padding no longer widens narrow ones (a 23-pixel `...`
  button was 30).
- The main menu bar is the same height (28) in the browser and on the
  desktop (it was 30 on Windows and Linux).
- A component's Left / Top / Width / Height are integers, as in RapidQ:
  `Height = ClientHeight / 2 + 20` stores 304, rounded half to even (the
  desktop kept 304.5).

## [2.104.0] — 2026-10-03

### Fixed
Found by running RapidQ's portable example programs on the native build and
the interpreter side by side (`tools/corpus_compare.mjs`):
- `REPLACE$(source, replacement, index)` is RapidQ's: it writes over the
  string at a position (`REPLACE$("Hello", "J", 1)` is "Jello"); it was
  Visual Basic's find-and-replace (that's REPLACESUBSTR$). RapidQ's PRINT
  USING formatter example and the Blocks game depend on it.
- Variables whose names differ only by type suffix are different variables,
  as in RapidQ (`i%` and `i$` side by side); a variable spelled like a
  function with another suffix (`day&` in `FUNCTION Day`) is not the
  function's result.
- A FOR loop's undeclared variable inside a SUB or FUNCTION is the
  routine's own on native builds too, as it was in the interpreter: two
  SUBs looping on `i` no longer move each other's counters (the Blocks game
  hung natively).
- A form's first OnPaint comes once its window shows (after OnShow, as
  Windows' WM_PAINT), when the screen's scale is known: what it draws is
  sharp on a high-DPI screen and the same on native and interpreted builds
  (text and lines drawn before were enlarged from 1×).
- `tools/corpus_compare.mjs` counts programs that wait with SLEEP as
  clock-dependent, and never touches the user's QREGISTRY store.

Portable corpus, native vs interpreted: 95 behave identically, 0 differ
otherwise; the rest depend on random numbers or the clock (12) or use the
network (16, not run).

## [2.103.0] — 2026-10-03

### Added
- RapidQ's POSTFIX (RPN) expressions from its manual (Appendix C): every
  operand and operator in its own parentheses, `(4) (7) (*) (4) (1) (-) (6)
  (^) (+)` = 757.
- `a NOT > b` (and `NOT <`, `NOT >=`, `NOT <=`) as RapidQ reads it — NOT
  binds looser than a comparison, so it's `NOT (a > b)` — as `a NOT= b`
  already was.
- A trailing comma leaves the last argument out (`AddItems " ",`); an
  argument left out adds no item to a list.
- `tools/rapidq_corpus.py` sorts RapidQ's examples by what they need
  (portable / Windows DLLs / DirectX / OLE / hardware ports / incomplete /
  not RapidQ): every portable one compiles.

### Changed
- Text after the end of a statement is reported in RapidQ's words:
  "Expected end-of-line but got …".

### Fixed
- `IF x THEN CALL Sub(1, 2): x = 0` and `… THEN CALL A ELSE CALL B`: a CALL's
  arguments end at `:` and ELSE.

## [2.102.0] — 2026-10-03

### Changed
- Components start with RapidQ's sizes, the same on every runtime (one
  table, `rapidr_value::layout::default_size`): QFORM 320 × 240, QBUTTON
  and QLABEL 75 × 25, QEDIT 120 × 25, QCOMBOBOX 145 × 25, QPANEL /
  QTABCONTROL / QGROUPBOX / QSCROLLBOX / QSTRINGGRID / QLISTVIEW 150 × 100,
  QCHECKBOX / QRADIOBUTTON 100 × 20, QCANVAS / QIMAGE 100 × 100,
  QPROGRESSBAR 250 × 25, … The desktop and the web had their own (a button
  was 80 × 25 on one, 100 × 30 on the other; a form 640 × 480).

## [2.101.0] — 2026-10-01

### Added
- QREGISTRY as RapidQ's manual has it (its own example runs as shown):
  `RootKey` (HKEY_CURRENT_USER by default), `OpenKey(Key, CanCreate)`,
  `CloseKey`, `CreateKey`, `DeleteKey` (with its sub-keys), `KeyExists`,
  `ValueExists`, `DeleteValue`, `RenameValue`, `MoveKey(Old, New, Delete)`,
  `KeyItem(i)` / `ValueItem(i)`, `KeyItemCount`, `ValueItemCount`,
  `HasSubKeys`, `CurrentKey`, `CurrentPath`, `GetDataType` / `GetDataSize`,
  `ReadString` / `ReadInteger` / `ReadFloat` / `ReadBinary` and their
  `Write…` twins. Paths are relative to the open key (`\` first: from the
  root), names ignore case, ReadBinary keeps RapidQ's index-from-−1 bug. The
  keys live in a per-user store, the same on every platform and nothing
  outside it touched: a Regedit-format text file in the user's settings
  folder (`RAPIDR_REGISTRY` names another), the page's local storage on the
  web. Native, interpreter and web.
- RapidQ's "Array of QREGISTRY is not supported!" (and the same for its other
  objects with no arrays: QMainMenu, QFileStream, QSOCKET, QRECT, …).

## [2.100.0] — 2026-10-01

### Added
- QFORM and QSCROLLBOX scroll as RapidQ's (Delphi's TScrollingWinControl),
  from one shared model (rapidr_value::scrollbars) that the desktop and the
  web draw and drive the same way: `AutoScroll` (on by default: a bar shows
  when the components reach past the client area), `HorzRange` /
  `VertRange`, `HorzPosition` / `VertPosition`, `HorzIncrement` /
  `VertIncrement` (8), `HorzMargin` / `VertMargin`, `HorzTracking` /
  `VertTracking`, `HorzVisible` / `VertVisible`. Scrolling moves the
  components (their Left / Top change, as Delphi's ScrollBy); a shown bar
  takes 17 pixels from ClientWidth / ClientHeight; an aligned component
  fills the scrolled area. The arrows move Increment pixels and repeat while
  held, a click in the track a page, the thumb drags (the components follow
  it with Tracking, else when it's let go), the mouse wheel scrolls; a click
  on a bar fires no OnMouseDown. A QSCROLLBOX has its sunken edge
  (bsSingle) with its components inside it. A QFORMMDI doesn't scroll its
  own components (its children's MDI area is Windows' own). Native,
  interpreter and web.

### Fixed
- Native builds named components after Rust keywords differently from the
  interpreter (`Box` was `box_`), so anything naming them (test hooks, a
  component found by name) missed them.
- The web showed the browser's own scroll bars on a form whose components
  didn't fit (the desktop none), and a QSCROLLBOX scrolled with the
  browser's / FLTK's bars without HorzPosition, VertPosition or the ranges.

## [2.99.0] — 2026-10-01

### Added
- QTABCONTROL as RapidQ's (Windows' tab control): a row of tabs over an
  area the program fills itself (no pages: OnChange shows and hides its
  components, as the manual's example does), from one shared model
  (rapidr_value::objects::tabcontrol) that the desktop and the web lay out
  and draw the same way — the tabs measured with the built-in fonts.
  `AddTabs`, `InsertTab(Index, Caption)`, `DelTabs(Index…)`, `Tab(i)` (read
  and set), `TabIndex`, `MultiLine` (rows, the selected one moved next to
  the area), `ScrollOpposite` (the rows before it moved to the other side),
  `TabPosition` (bottom / right), `VerticalTabs` (on the left or right,
  their captions turned), `ButtonStyle`, `FlatButtons`, `FlatSeperators`,
  `FocusButtons`, `HotTrack`, `TabWidth` / `TabHeight`, `TabInactiveColor`,
  `TabInactiveFont`, `Color`; a single line too long scrolls with its arrow
  buttons. Windows' rules: the first tab added is selected, a tab inserted
  before the selected one keeps it selected, deleting the selected tab
  selects none (-1); the program setting TabIndex doesn't fire OnChange, a
  click on a tab or the arrow keys do. ClientWidth / ClientHeight are the
  whole control; an aligned component fills the area inside the frame
  (TCM_ADJUSTRECT). Native, interpreter and web.

### Fixed
- QTABCONTROL's components were put on FLTK pages (one per tab) on the
  desktop and its tabs were plain buttons on the web; TabIndex, Tab(i),
  InsertTab, DelTabs and OnChange from the keyboard were missing.

## [2.98.0] — 2026-10-01

### Added
- QTRACKBAR as RapidQ's manual has it, from one shared model
  (rapidr_value::objects::trackbar) that the desktop and the web draw the
  same way (vector shapes, sharp at any scale): `Frequency`, `LineSize`,
  `PageSize`, `Orientation` (tbHorizontal / tbVertical), `TickMarks`
  (tmBottomRight / tmTopLeft / tmBoth, the thumb pointing at them),
  `TickStyle` (tsNone / tsAuto / tsManual), `SelStart` / `SelEnd` (the
  selection range shown in the channel) and `SetTick(Pos)`. The arrows move
  it by LineSize, Page Up / Page Down by PageSize, Home / End to the ends; a
  click beside the thumb moves it a page toward the click, and the thumb
  drags. OnChange fires when the user moves it. Native, interpreter and web.

### Fixed
- QTRACKBAR's defaults are RapidQ's (Max 10, PageSize 2, Frequency 1,
  150 × 45), not 0–100; Position stays within Min..Max; setting Position
  from the program now moves the thumb on the desktop.

## [2.97.0] — 2026-10-01

### Added
- QEDIT and QRICHEDIT keep their text and selection in one shared model
  (rapidr_value::objects::textedit), as RapidQ: `SelStart`, `SelLength`,
  `SelText` (read, and set to replace the selection), `Line(i)` (read and
  set), `LineCount`, `AddStrings` / `AddLines`, `Modified`, `WhereX` /
  `WhereY`, `SelectAll`, `ClearSelection`, `CopyToClipboard` /
  `CutToClipboard` / `PasteFromClipboard`, `ReadOnly`, `CharCase`. A
  multi-line edit's Text joins its lines with CR LF, as on Windows. What
  the user typed and selected is read from the widget before the program
  reads it, and the model is shown again after the program changes it.
  Native, interpreter and web.

### Fixed
- A QRICHEDIT read before its form showed (`Text`, `Line(i)`,
  `LineCount`) was empty.

## [2.96.0] — 2026-10-01

### Added
- QSTRINGLIST as RapidQ's manual has it (its own examples run as shown):
  `AddList(Other)`, `Parse(Source$, Delim$)` (the list becomes its pieces;
  returns how many), `Build(Start%, End%, Delim$)`, `Exchange(I%, J%)`, and
  `Duplicates` for a sorted list (dupIgnore, the default: a string already
  there isn't added again; dupAccept; dupError). Native, interpreter and
  web.

## [2.95.0] — 2026-10-01

### Added
- `ShowModal` returns the form's `ModalResult`, as RapidQ: setting it (to
  mrOk 1 … mrAll 8) closes the modal form; a button's `ModalResult` gives
  it to its form when clicked; closing the form otherwise gives mrCancel
  (2). QBUTTON `Kind` (bkOK, bkCancel, bkYes, bkNo, bkAbort, bkRetry,
  bkIgnore, bkAll, bkHelp, bkClose): its standard caption and ModalResult
  (bkClose closes the form). Native, interpreter and web.

### Changed
- Key events as RapidQ (Delphi) sends them: a form hears the keys typed in
  its controls only with `KeyPreview` on — and then before the control
  (they used to reach the form after the control, always). Within a key,
  every OnKeyDown comes before the OnKeyPresses. Desktop and web.

## [2.94.0] — 2026-10-01

### Added
- Menus as RapidQ has them, from one model for the desktop and the web
  (`rapidr_value::objects::menu`): QMENUITEM `ShortCut` ("Ctrl+N", "F2",
  "Shift+Del", … — the key works and shows in the menu), `Checked`,
  `RadioItem` (checking one unchecks the others), `Enabled` (greyed),
  `Visible`, `Hint`, `Command`, `MenuIndex` (setting it moves the item),
  `Count`; `AddItems` / `Insert` / `DelItems` / `DelIndex` on menus and
  items; separators ("-"); submenus at any depth. Every change shows at
  once (the menus used to be built once, without shortcuts or check marks).
  QPOPUPMENU `Popup(X, Y)` (screen coordinates), `OnPopup`, `Alignment`,
  and `AutoPopup`: a right click on a component whose `PopupMenu` is that
  menu. Native, interpreter and web.

## [2.93.0] — 2026-10-01

### Added
- RapidQ's QSOCKET as its manual has it: numbered sockets — `Sock% =
  S.Connect(Server$, Port%)`, `S.Open(Port%)` (a server), `ConnectionReady`,
  `Accept`, `IsServerReady` / `IsClientReady` (data waiting, without
  blocking), `Read` / `Peek` / `ReadByte` / `ReadLine`, `Write` /
  `WriteByte` / `WriteLine` (CR LF), `Close`, `GetPeerName`, `GetHostName`,
  `GetHostIP`, `MySocket` and `Transferred`. RapidR's own RSOCKET methods
  (Host / Port properties) are unchanged: the two are told apart by their
  arguments. Native and interpreter; a web page can't open TCP sockets, so
  there the calls fail as RapidQ reports it (-1, nothing ready).
- QFORM `AddBorderIcons` / `DelBorderIcons` (biSystemMenu, biMinimize,
  biMaximize, biHelp): the web greys out the title bar's buttons; the
  desktop keeps the set (its title bar is the system's — RapidQ's manual
  allows an icon to stay, greyed out).

## [2.92.0] — 2026-10-01

### Changed
- Arrays as RapidQ has them ("There is no checking for limits on arrays"):
  an index past one dimension is the next row's element (arrays are stored
  by the last subscript), and past the whole array a read gives the type's
  zero and a write is dropped — memory-safe, where RapidQ would touch other
  memory. Programs from RapidQ's examples that stopped with "Subscript out
  of range" (3dcube, COLUMNS, dayfunction) run.

### Fixed
- QSTRINGGRID's OnDrawCell fires when the grid is on screen, as in RapidQ,
  not while its form is still hidden (it ran before the program created
  what the handler reads: QStringGridsTwoLinesBitMap). Desktop and web.
- macOS: a form with a main menu shown after another form closed crashed in
  FLTK's "Window" menu (msweep.bas); RapidR's menu bar has no Window menu
  (RapidQ has none).
- Tests never reach a real printer: `tools/regress.sh` sends documents to
  PDFs, and the test calling every shared built-in (LFLUSH among them)
  prints nowhere.

## [2.91.0] — 2026-10-01

### Added
- `LPRINT` (as PRINT, `;` and `,` included) and `LFLUSH`, as RapidQ: the
  text goes on printer pages (A4, 10 pt) printed at LFLUSH, or when the
  program ends — through the same printing as `Printer.EndDoc` (the
  system's printer on the desktop, the print dialog on the web,
  `RAPIDR_PRINT_TO` a folder or file). Native, interpreter and web.
- Native builds: a line label or GOSUB inside WITH or CREATE (it used to be
  a build error there; the interpreter already had it).

### Fixed
- Inside a CREATE, a property read bare is the object's, as in RapidQ:
  `Left = (Screen.Width - Width) \ 2`, `PRINT ItemCount` (they read an
  empty variable before) — unless the program has a variable, constant,
  routine or component of that name. Native, interpreter and web.
- An undeclared variable that is only read is RapidQ's DOUBLE 0 (`PRINT zz`
  printed nothing). RapidQ's include folder: 81 of 108 libraries compile.

## [2.90.0] — 2026-10-01

### Added
- RapidQ built-ins that were missing (the corpus now compiles 158 of 386): `QUICKSORT(A(first), A(last),
  ASCEND|DESCEND)` (sorts that range of the array in place, numbers by
  value, strings by character; with or without parentheses), `PRINT TAB(n)`,
  `ATAN` (ATN), `GET$(n)` (bytes from standard input, for CGI programs),
  `SETCONSOLETITLE` (the terminal's title; the page's on the web) and
  `CHDRIVE` (Windows). Native, interpreter and web.

### Fixed
- Native builds call the program's own FUNCTION or SUB named like a
  built-in (`FUNCTION Get$`), as RapidQ and the interpreter do.

## [2.89.0] — 2026-10-01

### Added
- RapidQ syntax from its examples (the corpus now compiles 155 of 386):
  `STRUCT name … END STRUCT`, a user-defined type as TYPE; `""` inside a
  string is a quote (`"[:"":>"` is `[:":>`); BYTE / WORD / DWORD suffixes
  on number literals (`0??`); a `_` line continuation inside a string under
  `$ESCAPECHARS ON` (`"Accept: _` … on the next line).

## [2.88.0] — 2026-10-01

### Changed
- INKEY$'s extended keys (arrows, Home/End, Page Up/Down, Insert/Delete,
  F1–F12) are CHR$(27) + the QBasic scan code, as RapidQ's manual has them
  (chapter 6.5: "the first byte is an ESC character"), no longer QBasic's
  CHR$(0) + the scan code. Up is still `RIGHT$(k, 1) = "H"`.

### Added
- `$OPTION INKEY$ TRAPALL` / `$OPTION INKEY$ DEFAULT`: INKEY$ also returns
  Shift, Ctrl, Alt, Caps / Num / Scroll Lock and the menu key (CHR$(27) +
  their scan code: 42, 29, 56, 58, 69, 70, 93), switched on and off
  anywhere — in the program's windows and the page (a terminal can't tell
  these keys). Native, interpreter and web.

## [2.87.0] — 2026-10-01

### Changed
- A property RapidQ's manual lists as read-only (`Handle`, `ItemCount`,
  `SelCount`, `ColumnsCount`, `VisibleRowCount`, a stream's `EOF` / `Size`
  / `LineCount`, …) can't be assigned, in RapidQ's words: `Property
  ItemCount of List is read-only.` — by name or inside its CREATE (the
  assignment used to be ignored). None of the 386 RapidQ examples or its
  include libraries trips it. Native, interpreter and web.

## [2.86.0] — 2026-09-30

### Changed
- More of RapidQ's compile-time checks, in its compiler's words (they used
  to pass silently: extra arguments were dropped, missing ones empty, a
  second DIM reset the variable):
  `Too many actual parameters for S`, `Too few parameters for S` (a
  `DECLARE SUB`'s own signature is accepted too, as RapidQ does),
  `Identifier a already used, try another name` (`i%` and `i$` are two
  names), `Trying to assign return value while not in FUNCTION`. None of
  the 386 RapidQ examples or its include libraries trips them. Native,
  interpreter and web.

## [2.85.0] — 2026-09-30

### Added
- `$TYPECHECK ON` / `$TYPECHECK OFF` and `$OPTION EXPLICIT`, as RapidQ: while
  on, a variable stored into (`x = …`, `FOR x`, `INPUT x`) must be declared
  first (DIM, CONST, a parameter; `n%` is the DIMmed `n`), or it is RapidQ's
  `Undeclared identifier x`. It can be switched on and off around parts of
  a program. None of the 159 RapidQ examples that use it, nor its include
  libraries, trips it. Native, interpreter and web.

## [2.84.0] — 2026-09-30

### Changed
- Undeclared variables are DOUBLE, as in RapidQ (a variable without a
  suffix or declaration is a DOUBLE there), no longer
  VARIANT: `n = 7 / 2` is 3.5 as before, but a string can't go into one.
  `$OPTION DIM VARIANT` brings back the old behavior; `DIM v` without AS is
  still a VARIANT (RapidQ manual, DIM).
- RapidQ's compile-time type check: a string stored into a numeric
  variable (`DIM n AS LONG : n = "12"`, or an undeclared `x = "hi"`) is an
  error, in RapidQ's words: `Type mismatch, expecting type LONG, but got
  STRING` (it used to convert silently). VAL converts. None of the 386
  RapidQ examples trips it. Native, interpreter and web.

## [2.83.0] — 2026-09-30

### Added
- `INPUT$(n)`: waits for n keys (not echoed) and returns them the moment
  the n-th is pressed, as RapidQ — no polling: a console program sleeps in
  the terminal (or reads n characters from a pipe, returning what came if
  it ends first), a program with windows serves its events (timers,
  repaints) until the keys come, and the web suspends the program until the
  page's keydown. Native, interpreter and web.

## [2.82.0] — 2026-09-30

### Added
- RapidQ syntax from the example corpus (now 153 of 386 compile):
  a keyword as a variable declared by DEFSTR/DEFINT/… (`DEFSTR return`,
  qcgi.inc; `DIM step` stays an error); an array
  field without bounds (`Hint() AS STRING`, QtoolBar.inc: room for 0–255,
  REDIM for another size).
- `$OPTION DIM type` (BYTE … STRING, VARIANT): variables the program never
  declares, and a DIM without AS, are of that type; `$OPTION DECIMAL ","`
  (or a character code) is the decimal character VAL reads from then on.
  Native, interpreter and web.

### Fixed
- Inside a component's CREATE, its own indexed members (`Panel(0).Width`
  of a QSTATUSBAR, a QLISTVIEW's `Column(i)`, …) are the object's even when
  the program has an array of that name.

## [2.81.0] — 2026-09-30

### Added
- Templates (manual 10.8) on both backends: `TYPE Holder<DataType, Size>`
  … `END TYPE` and `DIM A AS Holder<INTEGER, 10>` — a TYPE made for each
  set of arguments, the parameters replaced in its fields' types and sizes
  and in its code (parameters and FUNCTIONs taking one too).
  QStringGridEx.inc compiles now.
- `Obj.Inherit<Event>` (manual 10.4): a TYPE's own `EVENT OnClick`, after
  the program gave the object its own handler, runs from that handler
  (`C.InheritOnClick`).

### Fixed
- Custom events (manual 10.9): a SUB given to an `AS EVENT(Template)`
  field is stored as its pointer (`> 0`, fired with CALLFUNC); it was
  taken for a component's event and the field stayed 0.
- Web: a component whose event handler was given twice (a TYPE's EVENT,
  then the program's own) ran it twice per event.

## [2.80.0] — 2026-09-30

### Added
- `INKEY$` on native, interpreter and web — the next key pressed, or ""
  without waiting, as QBasic's: a character, Enter CHR$(13), Escape
  CHR$(27), and the arrows, Home / End / Page Up / Page Down, Insert /
  Delete and F1–F12 as CHR$(0) + their scan code. A console program reads
  its terminal (a key at a time, no echo; INPUT gets line mode back), a GUI
  program the keys pressed in its windows, the web the keys pressed in the
  page (not those typed into fields). It was an unknown variable ("").
- `DOEVENTS` on the desktop lets the program's windows, events and timers
  run (it did nothing); on the web it pauses the program so the browser
  goes on — a `DO … DOEVENTS … LOOP` no longer freezes the page.

### Fixed
- **`SLEEP` counts seconds**, as RapidQ's manual says (`SLEEP 1.5`: one and
  a half seconds); it took milliseconds and dropped fractions, so `SLEEP 1`
  hardly paused. On the web it now really waits (the browser goes on).
- A QTIMER enabled again after being disabled ticks again; its Interval
  is read at every tick; timers run in a DOEVENTS loop too, not only once
  a form is shown modally.

## [2.79.0] — 2026-09-30

### Added
- `$OPTION ICON "app.ico"`: the program's icon, built into it and made
  the application's (every form without its own shows it) on native,
  interpreter and web; an icon that isn't there leaves the default one
  instead of failing the build.
- `$OPTION BYREF`: from that line on, parameters without BYVAL are passed
  by reference (RapidQ's default is BYVAL), on both backends.

## [2.78.0] — 2026-09-30

### Added
- QSTRINGGRID on native, interpreter and web: goColMoving / goRowMoving
  (dragging a fixed row's cell moves its column, a fixed column's cell its
  row — cells, width / height, column style and list with it; the selected
  cell follows), `VisibleRowCount` / `VisibleColCount` (rows / columns shown
  whole), and RapidR's `MoveCol From, To` / `MoveRow From, To`.
- `tools/real_input.py`: `d:x1,y1,x2,y2` drags.

## [2.77.0] — 2026-09-30

### Added
- **QLISTVIEW as RapidQ has it**, on native, interpreter and web alike:
  the control is laid out and painted by the shared model (sharp on
  high-DPI screens) instead of a plain text list / an HTML table.
  - `ViewStyle`: vsIcon (RapidQ's default: large icons with captions under
    them), vsSmallIcon, vsList (columns) and vsReport (rows under a column
    header); `LargeImages` / `SmallImages` / `StateImages`, `ImageIndex` /
    `StateIndex`.
  - `CheckBoxes` (click the box or press Space; `Item(i).Checked`),
    `MultiSelect` (Ctrl / ⌘-click, Shift-click, Ctrl+A; `SelCount`,
    `Selected(i)`), `SortType` stText (kept sorted by caption), `RowSelect`,
    `GridLines`, `HotTrack`, `HideSelection`, `BorderStyle`.
  - The header: its buttons fire OnColumnClick (`ColumnClick`), its edges
    resize the columns; scroll bars, the mouse wheel, and the keyboard
    (arrows, Home / End, Page Up / Down).
  - In-place caption editing when `ReadOnly` is False (as Windows): F2, or
    a click on the item already selected.
  - OnChange (Index, Change: ctText 0 / ctState 2) for each item whose
    selection, check or caption the user changed, then OnClick /
    OnDblClick.

### Changed
- A QLISTVIEW without `ViewStyle` now shows RapidQ's default, the icon
  view (it always looked like a report before); items have no state image
  until `StateIndex` is set (-1).

## [2.76.0] — 2026-09-30

### Added
- JPEG pictures wherever a bitmap goes (QBITMAP, QIMAGE, QIMAGELIST,
  `$RESOURCE`, `Draw`), decoded by the shared model on native,
  interpreter and web (jpeg-decoder, already part of the build).
- QIMAGELIST `AddICOFile` / `AddICOHandle` / `InsertICOFile` /
  `InsertICOHandle` (an icon is scaled whole to the list's size, its
  see-through parts kept) and `GetICO`; QIMAGE `ICOHandle` / `Icon`.

### Fixed
- A SUB / FUNCTION parameter or local named like a global component (`b`
  while a QBITMAP `B` exists; names are case-insensitive) is the variable
  in its routine, not the component — on both backends.

## [2.75.0] — 2026-09-30

### Added
- RapidQ's dotted TYPE fields (CommCtrl.inc, qdataBaseSQL.inc):
  `hdr.hwndFrom AS LONG`, `Table.Name(150) AS STRING`, nested as deep as
  written, are a record inside the record — `N.hdr.hwndFrom`,
  `N.Table.Name(2)` — on both backends.

### Fixed
- High-DPI on the web: a QSTRINGGRID's owner-drawn cells (OnDrawCell)
  are drawn at the screen's scale — lines, text and images sharp at 2×.

## [2.74.0] — 2026-09-30

### Added
- QTREEVIEW on native, interpreter and web: `StateImages` (a node's
  `StateIndex` image beside its own; index 0 is none, as in Windows),
  OnGetImageIndex / OnGetSelectedIndex (Index) — the program sets
  `Item(Index).ImageIndex` / `.SelectedIndex` as nodes are shown, asked
  again when the shown nodes or the selection change — `HideSelection`
  (no selection shown while the tree hasn't focus), and `GetItemAt(X, Y)`
  (the manual's hot-tracking example).

### Fixed
- Setting a tree's `Images` after it was shown now shows the icons.
- The web no longer warns about QIMAGELIST as an unknown component.

## [2.73.0] — 2026-09-30

### Added
- Window icons on native, interpreter and web: `Form.Icon` (a file) /
  `Form.IcoHandle` (a `$RESOURCE`), and `Application.Icon` / `IcoHandle`
  for every form without its own — an ICO, BMP, PNG or SVG. The web shows
  them in the forms' title bars and as the page's icon. (macOS shows no
  window icons; Windows and Linux do.)
- ICO and PNG pictures wherever RapidQ takes a bitmap (QBITMAP, QIMAGE,
  QIMAGELIST, `Draw`, `$RESOURCE`), decoded by the shared model with their
  see-through parts — every icon of RapidQ's own icon folder reads.

### Fixed
- `CopyRect(D, Image, S)` with `DIM R AS QRECT` rectangles copied nothing
  (a DIMmed QRECT had no value to pass).
- A QIMAGE whose size the program hasn't set takes its first picture's
  size, as RapidQ's manual says (it stayed 100 × 100).

## [2.72.0] — 2026-09-29

### Added
- **QHEADER** on native, interpreter and web: column headers the user
  clicks and resizes. `AddSections`, `Clear`, `SectionsCount`, and
  `Sections(i).Caption` / `Width` / `MinWidth` / `MaxWidth` / `Alignment` /
  `AllowClick` / `Style`; dragging a section's edge (the resize cursor
  shows there) fires OnSectionTrack (Index, Width, State: begin, move,
  end) and then OnSectionResize; a click fires OnSectionClick. Sections
  with `Style = hsOwnerDraw` are drawn by OnDrawSection (Index, Pressed,
  Rect), with the header's own canvas methods (`Sender.FillRect`, …).

## [2.71.0] — 2026-09-29

### Added
- **QOUTLINE** on native, interpreter and web — Windows 3.1's tree, shown
  as a tree view: `AddLines` (each leading space a level deeper),
  `AddChild(Index, S)`, `Insert`, `DelLines`, `Item(i)` read and written,
  `Row`, `LineCount`, OnClick / OnDblClick.
- **QFILEDIALOG** (RAPIDQ2.INC's): `Mode` fdOpen / fdSave, `MultiSelect`,
  `Files(0)` the folder then the picked names, `SelCount`, `FileTitle`,
  `DefaultExt`, `WarnIfOverWrite`, `Caption`.
- Web file dialogs: Open / Save now wait for the user (as on the desktop)
  in a dialog in the page listing the program's files, with a name field
  and Upload… for a file from the computer.
- Desktop test hook `RAPIDR_TEST_FILE_DIALOG` (what the dialogs answer).

### Fixed
- QOPENDIALOG / QSAVEDIALOG: RapidQ's `Filter` ("Pictures|*.bmp;*.ico|All
  Files|*.*") and `FilterIndex` work (the filter went to the dialog
  unconverted); `InitialDir`, `Caption`, a preset `FileName`; the web's
  Open returned True before anything was picked.
- The desktop test actions `__node_i` / `__toggle_i` fire OnClick as a
  click does.
- `tools/regress.sh` keeps its build folder bounded (it grew ~35 GB per
  run and filled the disk) and reports a web test that fails by its exit
  code; the IDE project-reload test counts forms, not their controls.

## [2.70.0] — 2026-09-29

### Added
- **High-DPI screens** (Retina, a browser at 2×) on native, interpreter
  and web: canvases, form surfaces, QIMAGE pictures, owner-drawn list and
  combo items, grid images and tree icons are shown at the screen's scale
  instead of enlarged — text, diagonal lines and ellipses are drawn from
  the device pixels, SVGs at the scale (drawn again when the scale is
  learned or changes). The pixels a program reads and saves (`Pixel`,
  `.BMP`, flood fills, SaveToFile) are exactly the 1× ones: each bitmap
  keeps what the screen shows next to them. `RAPIDR_SCALE` forces a scale
  on the desktop (tests); the whole desktop GUI suite and both web suites
  pass unchanged at 2×, and `tools/regress.sh` runs the web GUI suite at
  2× too.
- An image that is an SVG's (not drawn on) hands out the SVG itself as its
  `.BMP` / `GetBMP` data, so what it's drawn onto can draw it sharply.

## [2.69.0] — 2026-09-29

### Added
- `RUN "program"`: starts a program without waiting (its process ID), on
  native and interpreted desktop builds (the browser starts none).
- `INITARRAY(A, v1, v2, …)`: the first elements of A get the values (both
  backends).
- A method of any object expression (`This.Names.Item(2)`, a sub-object's
  `printer.Font.DelStyles(3)`); SUBI / FUNCTIONI closed by `END SUB` /
  `END FUNCTION` (QAVI.inc).
- Include folder: 85 of 108 libraries compile; example corpus: 148 of 386.

### Fixed
- SHELL / SHELLWAIT / RUN run the command through `cmd /C` on Windows
  (they used `sh -c` everywhere).

## [2.68.0] — 2026-09-29

### Added
- RapidQ syntax used by its include libraries and example programs, on
  both backends: an object field's own properties (`P.MoverRect.Top`);
  keywords as a TYPE's field and method names (`Step AS DOUBLE`,
  `Data AS QStringGrid`, `FUNCTION Create`, `SUB Close`), as parameters
  (`select`, `case` — SELECT CASE keeps working) and as variables
  (`type = 2`); `END PROPERTY SET`; `STRUCT … END STRUCT` (a TYPE);
  `ByVal` in a call's arguments; `_` stuck to a name at the end of a line
  continues it; comment lines inside a continued statement.
- RapidQ's include folder: 83 of 108 libraries compile on their own (72
  before); the example corpus: 140 of 386 programs (135).

## [2.67.0] — 2026-09-29

### Added
- **SVG images** wherever RapidQ takes a bitmap — QBITMAP and QIMAGE
  (`LoadFromFile`, `BMP`, `BMPHandle`), QIMAGELIST (`AddBMPFile`,
  `AddBMPHandle`), `Canvas.Draw`, `$RESOURCE` — on native, interpreter
  and web: drawn by resvg (pure Rust, Apache/MIT) in the shared image
  model, with soft edges (each pixel's opacity is kept, blended when
  drawn, and survives `.BMP` as a 32-bit BMP). RapidQ's BMP handling is
  unchanged.

## [2.66.0] — 2026-09-29

### Added
- QPANEL `BevelOuter`, `BevelInner`, `BevelWidth`, `BorderWidth` drawn as
  RapidQ does (a raised outer bevel by default) on desktop and web, from
  one shared model (`rapidr_value::objects::bevel`).
- `examples/digdisplay`: a clock on RapidQ's QDigDisplay.inc, with
  RapidR's own seven-segment bitmaps (the originals were never
  distributed; `tools/make_digit_bitmaps.py` makes them).

### Fixed
- RapidQ's include libraries (QBevel.inc, QDigDisplay.inc, …), both
  backends: an instance of a TYPE extending a component created inside a
  form stopped with "not an object"; `.Field = x` in its PROPERTY SET was
  lost; fields redeclaring the component's properties (`Width AS LONG`)
  never reached the widget; FOR counters and `Result` in its code were
  taken for the component's properties.
- A canvas setting its own size in its OnPaint (QDigDisplay) repainted
  forever: a size change repaints only when the size really changes.

## [2.65.0] — 2026-09-29

### Added
- **QTREEVIEW in-place editing** on native, interpreter and web: F2, or a
  click on the node already selected (a double click doesn't), asks
  `OnEditing (Index, AllowEdit)` — which may refuse — and opens an editor
  over the node's text; Enter or leaving it asks `OnEdited (Index, S)`,
  whose `S` answers back the node's new text; Escape drops the edit, and
  `ReadOnly` allows none. Test actions `tree.__edit` / `__enter` /
  `__escape` (desktop hooks and the web harness); fixture `tree_edit.bas`.
- **RapidQ's global objects** from one shared model
  (`rapidr_value::globals`) on native, interpreter and web — they were
  property bags, so `Screen.Width` read 0: `Screen` (`Width`, `Height`,
  `ClientWidth` / `ClientHeight`, `MouseX` / `MouseY`, `Monitors`,
  `Cursor` over every form), `Application` (`ExeName`, `Path`, `Title`,
  `Terminate`, `Minimize`, hint settings kept), `Clipboard` (`Text`,
  `SetAsText`, `GetAsText(n)`, `Clear`, `HasFormat(CF_TEXT)`,
  `FormatCount`, `Open` / `Close`) and `Mouse.X` / `Y`. The desktop uses
  the system clipboard (arboard, MIT/Apache); the web keeps the
  program's own and writes it to the browser's when allowed. Tests set
  `RAPIDR_TEST_CLIPBOARD` and never touch the user's clipboard.
- Component `Cursor` on the desktop (it was ignored), with RapidQ's codes
  (`crDefault` 0 … `crHandPoint` -21) shared by both runtimes.

### Fixed
- Web: `Cursor` used made-up numbers (1 = hand); it takes RAPIDQ.INC's
  `cr*` codes now.
- A `QRECT`'s (or any object's) `Left` / `Top` / `Right` / `Bottom` read
  0 until set (they were empty); on the web a DIM'd object's fields were
  not kept at all.

## [2.64.0] — 2026-09-29

### Added
- **QTREEVIEW** from a shared model (`rapidr_value::objects::tree`) on
  native, interpreter and web: nodes numbered depth-first as RapidQ does,
  `AddItems`, `AddChildItems`, `InsertItem`, `DelItems` (with the node's
  subtree), `Clear`, `Sort`, `Expand` / `Collapse` (recursive),
  `FullExpand` / `FullCollapse`, `GetItemAt`, `Item(i).Text` /
  `ImageIndex` / `SelectedIndex` / `StateIndex` / `HasChildren` /
  `Selected` / `Expanded` / `Count` / `Level` / `IsVisible` / `Parent` /
  `Handle`, `ItemCount`, `ItemIndex`, `TopIndex`, `ShowButtons`,
  `ShowLines`, `Indent`, `Images` (node icons), `LoadFromFile` /
  `SaveToFile` (tab-indented lines). What the user does asks the program
  first — `OnChanging (Index, AllowChange)`, `OnExpanding`,
  `OnCollapsing` — then `OnChange`, `OnExpanded`, `OnCollapsed`;
  `OnDeletion` for every deleted node; `OnClick`, `OnDblClick`. RapidR's
  older names (`AddRoot`, `AddChild` by text, `SelectedItem`) still work.
  It was a bare FLTK tree / HTML list with a few RapidR-only methods.

### Fixed
- Desktop: **a click on a button fired OnClick twice**, and every widget
  with its own event handling (buttons, check boxes, edits, grids, lists,
  canvases, splitters, MDI frames) also got FLTK's handling first: fltk-rs
  runs a widget's own handler before a custom one by default. RapidR's
  handlers run first now (`super_handle_first(false)`), as they were
  written to.
- Desktop: a click on an owner-drawn list box's scroll bar now scrolls it.

## [2.63.0] — 2026-09-29

### Added
- QLISTBOX `ExtendedSelect` (on by default, as in RapidQ): in a
  MultiSelect list, Shift+click selects a range, Ctrl+click toggles, a
  click selects one item; off, a click toggles. `TabWidth`: tabs in items
  go on to its stops (dialog units; 32 by default). In the shared list model
  (`ItemList::click`), for drawn lists on the desktop and the web.

## [2.62.0] — 2026-09-29

### Added
- **`$THEME` in interpreted programs** (only native builds honored it) and
  a `RAPIDR_THEME` environment variable for programs without one. Names:
  fltk-theme's themes (`classic`, `aero`, `metro`, `aquaclassic`,
  `greybird`, `blue`, `dark`, `highcontrast`) and schemes (`aqua`,
  `fluent`, `clean`, `gleam`, `svg`, `sweet`, `fleet1`, `fleet2`, in light
  colors), FLTK's (`base`, `gtk`, `plastic`, `oxy`), and platform names
  (`windows`, `mac`, `linux`, `win7`, …).

### Changed
- Linux (and other non-macOS / Windows systems) default to a light look
  (Gleam) instead of Dark: RapidQ programs set their colors for a light
  look. macOS (classic Aqua) and Windows (Metro) keep theirs.
- Native debug builds keep line tables only (`[profile.dev]`): full debug
  info made every program's build hundreds of megabytes.

### Fixed
- Native builds: a `$THEME` value with a quote in it broke (or injected
  into) the generated program; it is escaped.
- `$THEME crystal` crashed (fltk-theme's scheme panics): it gives `clean`.

## [2.61.0] — 2026-09-29

### Fixed
- Web: a `ShowModal` in the main program didn't wait — the statements after
  it ran at once. It waits for the form to close, as on the desktop; the web
  IDE shows the project's startup form after the program's own statements
  (as RapidQ's designer places it), not before them.
- Web: a program whose main body finishes with no form open ends (its
  timers stop, no event reaches it), as the desktop program exits.
- `Form.Repaint` (`Refresh`, `Update`, `Paint`) fires the form's OnPaint on
  the desktop (it was "not implemented") and the web.
- Web: a QPANEL's `Caption` read back the text of the controls inside it.

### Added
- Web GUI parity checks for a resized form and a dragged QSPLITTER
  (`align_layout`, through `rapidr_test_resize`), a ShowModal in the main
  program and Form.Repaint (`startup_modal`).

## [2.60.0] — 2026-09-29

### Added
- **QLISTBOX `Columns`**: items flow down each column and into the next,
  that many columns showing, scrolled sideways; the arrow keys move within
  and across columns. Owner-drawn lists can have columns too (OnDrawItem's
  Rect is the item's cell).
- **Owner-drawn QCOMBOBOX** (`Style = csOwnerDrawFixed / csOwnerDrawVariable`):
  OnDrawItem draws each item (FillRect, TextOut, Draw, … on the combo box),
  OnMeasureItem sizes them (Variable); the box shows the selected item as
  drawn, the drop-down lists the items as drawn, a pick sets ItemIndex and
  fires OnChange, Up / Down pick the item before / after.
- Both from the shared list model on native, interpreter and web
  (`ItemList::item_rects`, `item_at`, `set_view`).

### Fixed
- Desktop: QSTRINGGRID and owner-drawn QLISTBOX scroll bars weren't drawn
  (FLTK's table left them unpainted under the theme); an owner-drawn list
  box's background showed the theme's shading past its items; clicking an
  owner-drawn list box redrew only part of it.

## [2.59.0] — 2026-09-29

### Added
- **Keyboard events on the desktop** (there were none): `OnKeyDown(Key,
  Shift)` and `OnKeyUp(Key, Shift)` with the Windows virtual-key code
  (`A` = 65, arrows 37–40, F1 = 112, …) and RapidQ's Shift state (`ssShift`
  256, `ssCtrl` 16, `ssAlt` 1); `OnKeyPress(Key)` with the character typed
  (Enter 13, Backspace 8, Tab 9, Escape 27). They go to the focused
  component, then to its form — the same on the web, where a click lets
  a form or canvas take the keyboard.
- **Mouse events on every component** with RapidQ's arguments: 
  `OnMouseDown` / `OnMouseUp(Button, X, Y, Shift)` (`mbLeft` 0, `mbRight`
  1, `mbMiddle` 2) and `OnMouseMove(X, Y, Shift)`, to the component under
  the mouse (the one pressed while a button is held), X and Y in it. On the
  desktop only QCANVAS (with just X, Y) and QIMAGE had them.
- One set of rules for both runtimes (`rapidr_value::input`: key codes,
  typed characters, Shift bits, argument lists).
- The web IDE's handler stubs use RapidQ's parameters (OnClose's Action,
  keys, mouse, OnSelectCell, OnMeasureItem, …).

### Changed
- Web: key events passed (keyCode, shift, ctrl, alt) and mouse events (X,
  Y, Button) — five values, so a `Sender` parameter got the wrong one; a
  form also got the mouse events of its controls. The examples
  (`web_canvas.rr`, `web_ide.rr`, `strip_ide.rr`) use RapidQ's order.

### Fixed
- Desktop: clicking a QSTRINGGRID's header left the grid blank but for a
  few cells (FLTK's table handles the click first and redrew only part of
  itself); the grid is drawn whole after a click.
- Desktop: redrawing a form's drawing surface painted over its controls.
- Web: an owner-drawn QLISTBOX showed its items as they were before
  OnDrawItem ran (a newly selected item didn't show as selected).
- The `event_answers` test gives its list items readable heights.

## [2.58.0] — 2026-09-28

### Added
- **Event parameters come back to the runtime**, as in RapidQ (which passes
  them by reference), on native builds, the interpreter and the web:
  - QFORM `OnClose(Action)`: `Action = caNone` (or `False`) keeps the form
    open, `caMinimize` minimizes it; it starts as `caHide`;
  - QSTRINGGRID `OnSelectCell(Col, Row, CanSelect)`: `CanSelect = 0` puts
    the selection back (also from a TYPE's `EVENT OnSelectCell`);
  - QLISTBOX `OnMeasureItem(Index, Height)` (`lbOwnerDrawVariable`): each
    item is as tall as its handler says; OnDrawItem's Rects follow;
  - QSTRINGGRID `OnListDropDown(Col, Row, S)`: a gcsList column's
    drop-down shows the items the handler leaves in `S`.

  The continuation of an event (v2.57.0) now gets the arguments as the
  handler left them: the VM hands over the handler's parameters when it
  returns; native builds bind handlers so their parameters write back
  (`rp_bind_event_out`; a SUB bound to an event takes its parameters by
  reference from the runtime, still by value when the program calls it).
  One handler type and calling rule for both runtimes
  (`rapidr_value::events::Handler`, `call`).

### Fixed
- Desktop: the window's close button fired no OnClose (FLTK hid the window
  itself; Escape closed forms too); `Form.Hide` fired OnClose, which only
  `Close` does.
- Web: `Form.Caption` read back the whole window's text (title bar
  buttons, controls); it reads the title.

## [2.57.0] — 2026-09-28

### Added
- **QFORMMDI** (RAPIDQ2.INC's MDI form) on the desktop and the web: child
  windows inside the form, each showing one of the program's components
  (`AddChild(Edit(i).Handle, "Title", i, left, top, width, height,
  DefaultSize)`), with a title bar, minimize / maximize / close buttons,
  moved by the title bar and sized by the corner, double-click to
  maximize; `CloseChild`, `CloseAllChild`, `CascadeChild`, `SetHorzChild`,
  `SetVertChild`, `IconArrangeChild`, `MinimizeAllChild`,
  `MaximizeAllChild`, `RestoreChild`, `ActiveNextChild`,
  `ActivePreviousChild`, `ActiveChild`, `GetChild`, `ChildExist`,
  `FreeChild`; `ChildCount`, `ChildMax`, `ChildCaption`, `ChildHandle`,
  `ChildLeft` / `Top` / `Width` / `Height`, `ChildState`,
  `ComponentIndex`, `ChildResult`; `OnChildActive`, `OnChildClose` (setting
  `ChildResult` to False keeps the child open), `OnChildResize`. One model
  (`rapidr_value::mdi`, the same code applies it in both runtimes); it was
  a stub (AddChild only counted) on the desktop and nothing on the web.
- **`Component.Handle`**: a stable number for each component (it was
  empty), and the way back from it (`rapidr_value::handles`).
- **Events whose handler the runtime waits for** (`rp_fire_event_then`,
  `rapidr_value::events`): the runtime goes on once the handler has run —
  at once in a native build; in the interpreter the continuation travels
  with the queued event and the VM hands it back when the handler returns,
  even if it waited for a dialog. OnChildClose's `ChildResult` uses it; the
  by-reference event results (OnClose's Action, …) will.

### Fixed
- Desktop: a component given a parent after its form is shown (`Late.Parent
  = Form` in an event handler) got no widget; it gets one then, with its
  children (the web did this already).
- The interpreter runs the events a host operation queued until none is
  left (events a handler's continuation fires included).

### Tests
- **GitHub Actions no longer run on push** (they cost minutes and failed on
  Linux): the CI workflow runs only when started by hand; every check runs
  locally before a commit — `tools/regress.sh` (unit, conformance on both
  backends, native examples via `tools/native_examples.sh`, desktop GUI
  events, the web suites).
- GUI parity (desktop native + interpreted + browser): `mdi_children`
  (adding, tiling, next, closing with a veto, FreeChild / GetChild /
  ChildExist, the components' places), `late_parent`; the test hooks read
  `name.__shown` (a widget the user can see). Unit tests for the MDI model
  and the event continuations.

## [2.56.0] — 2026-09-28

### Added
- **Memory functions, memory-safe, on every backend**: `VARPTR`,
  `UDTPTR`, `VARPTR$`, `MEMCPY`, `MEMSET`, `MEMCMP`, `RTLMOVEMEMORY`,
  `SIZEOF`, `QMemoryStream.Pointer`, `@x` passed to a DLL — natively,
  interpreted and in the browser (`rapidr_value::memory`,
  `rapidr_ast::memory`). An address is an ordinary number (it fits a LONG;
  `ptr + 4` works) inside a live view of the program's own data: an array's
  elements, a TYPE's fields, a stream's buffer, or a plain variable's bytes
  (copied back into the variable after a statement that writes memory).
  Bytes are laid out as RapidQ stores them: BYTE 1, WORD / SHORT 2,
  INTEGER / LONG / DWORD / SINGLE 4, DOUBLE 8, `STRING * n` n bytes, a
  STRING as the address of its characters, TYPE fields packed. An address
  that isn't the program's memory, runs past the end of it, or belongs to
  something that no longer exists is a run-time error, never a crash. The
  interpreter refused these, and native builds returned 0 for `VARPTR`.
- **`Stream.WriteUDT` / `ReadUDT`** write and read a TYPE's bytes (the
  manual's `S AS STRING*8, N AS INTEGER` is 12 bytes).
- **Native DLL calls get real memory**: an argument that is an address the
  program got (`VARPTR`, a stream's `Pointer`, a TYPE) is passed as a real
  buffer holding those bytes, and what the DLL writes there is copied back
  (checked with the C library's `strlen` / `memset`).

### Fixed
- `SIZEOF(INTEGER)` is 4 (it was the length of the text "INTEGER", 7),
  `SIZEOF(SHORT)` 2, `SIZEOF(TMyType)` the TYPE's packed size, a STRING
  variable its length.
- The web IDE's help for `VARPTR` / `VARPTR$`, and entries for `UDTPTR`,
  `MEMCPY`, `MEMSET`, `MEMCMP`, `SIZEOF`, `RTLMOVEMEMORY` and `CBOOL`.

### Tests
- Conformance `memory_functions` (the manual's examples: variables, strings,
  arrays, TYPEs, streams, a SUB swapping two variables through pointers);
  unit tests for the layout, copies and bad addresses.
- 136 of 386 example programs compile (was 134), all build natively.

## [2.55.0] — 2026-09-28

Closing open items before adding new syntax: an audit of the roadmap's
compatibility list, every fix made on native, interpreted and web together.

### Changed
- **One builtin library for all three runtimes**: the 63 string / math /
  conversion builtins that the desktop and web runtimes each had a copy of
  now live once in `rapidr_value::builtins` (about 900 duplicated lines
  gone), so a fix reaches native builds, the interpreter and the browser at
  once.
- **Numbers print as RapidQ shows them** (Delphi's `FloatToStr`, which the
  manual's `FORMAT$` / `STRF$` point to): 15 significant digits, so
  `0.1 + 0.2` prints `0.3`, `1 / 3` `0.333333333333333`, `2 ^ 70`
  `1.18059162071741E21`, `1E-7` stays short; `NAN` / `INF`.
- **QTIMER is enabled by default**, as the manual says: a timer with only
  `Interval` and `OnTimer` set now ticks (it never did, on any runtime).

### Fixed
- **`INSERT$(insert, source, index)`** takes RapidQ's argument order
  (`INSERT$("hi", "Hello", 3)` = `Hehillo`); it inserted the wrong way round.
- **`FORMAT$`** is Pascal's `Format` with any number of arguments: `%d %u
  %x` (`.prec` = at least that many digits), `%e %f %g %n %m`, `%s` (`.prec`
  = at most), width, `-`, `%1:d` argument index, `*`, `%%`. It formatted one
  number with a VB-like picture.
- **`STRF$(v, format, precision, digits)`** is `FloatToStrF` (ffGeneral,
  ffExponent, ffFixed, ffNumber); it ignored its arguments. Decimal digits
  round half away from zero, as in RapidQ (`%.1f` of 2.25 is `2.3`).
- **`RANDOMIZE seed`** repeats the sequence (it was ignored everywhere), with
  one generator on every runtime; the data-science components (`shuffle`,
  `rand`, `randn`, `choice`, …) draw from it too.
- **Type suffixes declare types** (manual): `?` BYTE, `??` WORD, `???`
  DWORD, `%` SHORT, `&` LONG, `!` SINGLE, `#` DOUBLE — `q% = 40000` holds
  -25536, `b? = 300` holds 44; `DIM n%`, parameters and `FUNCTION f%`
  without `AS` take the suffix's type. `b?` / `w??` didn't parse.
- **QCOOLBTN / QOVALBTN groups** (`GroupIndex`, `Down`, `AllowAllUp`):
  pressing one releases the others of its group, `Down = True` from the
  program too, the button that's down stays down unless `AllowAllUp`; OnClick
  reads the new `Down`. The desktop only toggled the pressed button, the web
  didn't toggle at all. One rule (`rapidr_value::toggle_group`) for both.
- Desktop buttons, cool buttons and images click only when the mouse went
  down on them and came up over them: a stray release (macOS can deliver one
  when a window appears) clicked a focused button twice. Space clicks a
  button once, when it's released; Enter at once.
- **`END` in the browser** stops the program as on the desktop — in the web
  interpreter and in compiled web builds (`rapidr build --web`, where END
  only logged and the program went on): what was written to files is kept,
  the forms close, timers stop, no event reaches the program any more.
- The web read `Enabled` (and other visual properties) of a component with
  no element, such as a QTIMER, as empty.

### Tests
- Conformance: `builtins_manual`, `type_suffixes`, `early_findings` (the
  first review's findings, all fixed), `inv_instr_redim`; GUI parity
  (desktop native + interpreted + browser): `coolbtn_group`,
  `timer_default`; `tests/web_end_timer.mjs` runs END in both web builds —
  the first test of a compiled web build.
- Desktop GUI tests ignore the real mouse and keyboard (only the test's own
  events drive the program): a key typed elsewhere during a run landed in
  the test window and clicked its focused button, making results vary.
- The desktop GUI test hook fires one click per turn of the event loop, as
  real clicks come (the interpreter runs handlers after the click returns);
  `node tests/native_gui_events.mjs <name>` runs one case.
- Builds are warning-free: unused code removed (the old QSTRINGLIST, a
  widget kind nothing created, unread fields), deprecated web-sys call.
  QFORMMDI, found to be a stub, is on the roadmap.
- `rand` is no longer a dependency.

## [2.54.0] — 2026-09-28

### Added
- **`DIM s AS STRING * n`** and `Name AS STRING * n` inside `TYPE`: a store
  is cut to `n` characters (RapidQ keeps a fixed string in an `n`-byte
  buffer; what a shorter value leaves unused isn't part of the text), for
  variables, array elements, TYPE fields and locals. Shared by both
  backends and the browser (`__to_fixed`, `rapidr_ast::numeric`).
  `STRING * 0` (an API buffer in old code) stays unbounded.
- **`CBOOL(x)`**: true for a non-zero number or numeric string, and for any
  other non-empty string. Interpreter, native and web.
- **`ON ERROR RESUME NEXT` / `ON ERROR GOTO label|0`** (VB code, not RapidQ)
  is accepted and ignored — a run-time error still ends the program — so
  such sources compile instead of failing on the first line.
- **A `WITH` left open** is closed by `END SUB` / `END FUNCTION`, as RapidQ
  allows.
- **Routines that differ only by type suffix** (`FUNCTION Day$` and
  `FUNCTION Day`) are different routines on both backends
  (`rapidr_ast::suffix_routines`); the interpreter used to run the last
  one for both, native builds didn't compile.
- **Native builds: `GOSUB` / labels inside `SELECT CASE`** (the selector is
  kept across states, each branch is a state). A label inside `WITH` or
  `CREATE` is still refused with a message.

### Fixed
- Native builds: a local variable sharing a global array's name (`month&`
  next to `DEFSTR MONTH$(1 to 12)`) made the array unknown in every routine
  after it (`cannot find value month`); a routine's own declarations no
  longer change what a name means elsewhere.

### Tests
- Conformance: `fixed_strings`, `on_error_accepted`, `with_unclosed`,
  `suffix_names`, `local_shadows_array`, `gosub_in_select` (interpreter and
  native); unit tests for the parser, `rapidr_value` and the renaming pass.
- 134 of 386 example programs compile (was 130) and all 134 build natively.

## [2.53.0] — 2026-09-28

### Added
- **`INPUT #n, a, b` and `LINE INPUT #n, s`** read from a file opened
  `FOR INPUT AS #n`: a field is a quoted string or text up to the next
  comma or line break; `LINE INPUT` takes the rest of the line. Both
  backends and the browser.
- **BASIC file I/O works in the browser**: `OPEN … FOR INPUT | OUTPUT |
  APPEND | BINARY AS #n`, `PRINT #`, `WRITE #`, `EOF`, `LOF`, `SEEK`,
  `CLOSE`, `KILL`, `FILELEN` use the page's own files (the same store the
  stream objects use); they were stubs, and `PRINT #` stopped the program.
  It is one shared implementation (`rapidr_value::basic_files`) for the
  interpreter, native builds and the web: a file is read whole when opened,
  and what's written goes back when it is closed, when its length is asked
  for, or when the program ends — a file the program never closed keeps its
  data (the desktop lost it before).
- `FREEFILE` without parentheses (`n = FREEFILE`) is the function, not an
  undeclared variable.
- **Keywords as names**: `DIM New AS QMENUITEM, Open AS QMENUITEM`,
  `Open.Caption = "&Open"`, `File.AddItems New, Open, Save`. A statement
  keyword (`Open`, `Close`, `Write`, `Seek`, `Kill`, `Input`, …) followed
  by `.`, declared with `AS`, or used as an argument is an identifier.
- **`CREATE cells(0 TO 9, 0 TO 4) AS QBITMAP … END CREATE`** makes an array
  of components, like the same `DIM`.

### Fixed
- **Native builds ignored `bups.OnPaint = bups.paint`** when the handler is
  a SUB with a dotted name (`SUB bups.paint`): the event was set to null, so
  the corpus's Sokoban (GB) drew only its buttons natively. Native and
  interpreted builds now draw the same window.

### Tests
- Conformance `basic_file_io` and `create_array` (both backends; the
  first also in the browser), `dotted_paint` (desktop and web), unit tests
  for the file module.

## [2.52.0] — 2026-09-28

The web IDE kept in step with the desktop, native and interpreter builds.

### Added
- **The conformance suite runs in the browser**: `node tests/web_conformance.mjs`
  compiles every console case of `tests/conformance/cases` with the web
  IDE's wasm compiler, runs it in the wasm VM, and compares with the same
  `.expected` file the interpreter and the native build are held to. 53 of
  55 match; two can't be compared in a browser (ANSI terminal codes, the
  file system) and carry a `' xfail: web — reason` marker.
- **`$RESOURCE` in the web IDE**: the files come from the project's assets
  (found by their last path part, `resource_files\two.bin` → `two.bin`), and
  are built into the program as on the desktop. A missing one is a compile
  error that names it. The wasm `compile()` takes the assets as a third
  argument.

- **The desktop's GUI fixtures run in the browser too**:
  `node tests/web_gui_parity.mjs` fires the same events on the same
  `tests/fixtures/*.bas` as `tests/native_gui_events.mjs` (both read the
  table in `tests/gui_parity_cases.mjs`) and compares the same properties
  with the same expected values. 21 checks match; two cases don't apply to a
  browser (a window resized by the user, a directory listing).
- `rapidr_get_prop(name, prop)` on the wasm module (and
  `window.__rapidr_rt` in the preview): a property as the program reads
  it, for tests and tools.

### Fixed
- **`ShowModal` waits on the web when a handler calls it**: a second form
  opened from a button handler now blocks that handler until the form
  closes (the VM suspends it, as the desktop's wait does) — the code after
  `Form2.ShowModal` ran at once in the browser before. Method calls can now
  suspend the VM like builtins do. The startup form's `ShowModal` in the
  main body still doesn't wait: the IDE puts it before the program's own
  statements.
- `Label.Caption` and the other captions read back what the program set;
  the browser returned the shown text with its `&` accelerator marks
  taken out (`"a & b"` came back as `"a  b"`).
- A new QLABEL / QBUTTON has an empty caption on the web as on the
  desktop (it was "Label" / "Button").
- **QSTRINGLIST was two different implementations**: the desktop's had
  `Item(i)`, the web's had `Get(i)` only, so RapidQ programs that call
  `Names.Item(1)` got "" in the browser. It is now one shared model
  (rapidr-value, the list box's items) both runtimes use, with `Add`,
  `Insert`, `Delete`, `Item(i)` read and write, `Count`, `IndexOf`, `Sort`,
  `Text`, `Clear`, `LoadFromFile` and `SaveToFile`.
- `Form.Font.Size = 12`, `Font.Name`, `.Bold`, `.Italic` and `.Color` set
  the flat font properties on the web too, as they do on the desktop, so
  text drawn on a form uses the size the program set.
- Conformance case `stringlist`, `resources_memory` and the web run above.

## [2.51.0] — 2026-09-28

RapidQ syntax that real programs use and RapidR refused. Of the 386 programs
in RapidQ's own example folder, 130 now compile (116 before); most of the
rest call Windows APIs, which RapidR doesn't emulate.

### Added
- **Whole arrays as arguments**: `Fill M(), 5`, `Total(M(), n)`,
  `CALL Sort(Names())` pass the array to a SUB / FUNCTION with an array
  parameter (`A() AS INTEGER`). `M()` was read as a call of a function `M`.
  A shared pass, so both backends agree.
- **Multi-dimensional array fields in a TYPE**: `vertex(9, 2) AS SINGLE`,
  `tag(1 TO 2, 0 TO 1) AS STRING` (the OpenGL, CGI and toolbar includes
  use them), on both backends.
- `IF c THEN: a: b: END IF` and `IF c THEN :a` with `ELSE :b` on the next
  line: a colon right after THEN starts a block. A statement after a colon
  may be a comment (`… : ' note`), and a redundant `END IF` may end a
  one-line IF.
- `=>` and `=<` for `>=` and `<=`.
- `&H1&`, `&HFFFF&`: hex literals with the long-integer suffix, as the
  Windows includes write them.
- `CASE 4, 7  C = -2`: the body may follow the case list without a colon.
- A line number in front of `NEXT`, `WEND` and the other ends of a block
  (`310 NEXT I`), and in front of `DATA` (`130 DATA 1, 2`).

### Fixed
- **Native builds now share undeclared variables like the interpreter.** A
  variable that is never DIMmed is global — one variable for the main
  program and every SUB / FUNCTION, kept between calls. Natively each
  routine got its own copy, so `q = 5` in a SUB never reached the main
  program and `w = w + 1` restarted at 0 on every call.
- A program that uses `pi` as a variable of its own (`3dcube.bas`) built
  natively with an error; `pi` is a constant only while nothing assigns it.
- **`$ESCAPECHARS ON` belongs to its own file.** A program that turned it
  on before `$INCLUDE "RapidQ2.inc"` broke the include's plain `""`
  strings; an include file now starts with it off, and the includer's
  setting comes back after it.

### Tests
- Conformance: `syntax_forms`, `array_refs`, `type_multidim_fields`,
  `implicit_globals`, `pi_variable`, and a preprocessor test for the
  `$ESCAPECHARS` scope.

## [2.50.0] — 2026-09-28

### Added
- **Owner-drawn list boxes**: QLISTBOX with `Style = lbOwnerDrawFixed` or
  `lbOwnerDrawVariable`, `ItemHeight` and `OnDrawItem(Index, State, Rect)`,
  on desktop and web, natively and interpreted. RapidQ's *OwnerDraw ListBox*
  example now looks as intended.
  - `FillRect`, `TextOut`, `Line`, `Rectangle`, `Circle`, `Pset` and `Draw`
    on the list draw the item they land on (`Rect` is the item's slot in the
    list); the items are made from the shared bitmap model, so the desktop
    and the web draw the same pixels, text in the built-in Liberation fonts
    and the list's `Font`.
  - `OnDrawItem` fires for every item when the list changes (items,
    selection, height): `State` is 0 for the selected item and 1 for the
    others, as RapidQ programs test it. An item nothing was drawn on shows
    plainly (the selected one white on blue).
  - Clicks and the arrow keys, Home, End, PageUp and PageDown select
    (OnClick / OnDblClick; a MultiSelect list toggles).
  - Every item is `ItemHeight` tall: `OnMeasureItem`'s answers aren't read
    yet (an event can't return a value), and combo boxes aren't owner-drawn.
- **330 constants of RapidQ's `RAPIDQ.INC` were missing** and silently
  evaluated to 0 or "": now `lbStandard` / `lbOwnerDrawFixed` /
  `lbOwnerDrawVariable`, `csOwnerDraw…`, `gcsList…`, every `go…` grid option,
  `ta…` alignments, `cr…` cursors, `ss…`, `vs…`, `bk…`, `ft…`, `sc…`, `os…`,
  the rest of the `cl…` system colors (their usual Windows values) and more.
  A test keeps names unique and checks a sample of values.
- Tests: a unit test for owner drawing, `tests/fixtures/owner_list.bas` (native
  and interpreted agree), `tests/web_ide_owner_list.mjs`.

### Fixed
- `clGreen` and `clPurple` had Delphi's values; RapidQ's are `&H00FF00` and
  `&HFF00FF`.

## [2.49.0] — 2026-09-28

### Added
- **Drawing on a QFORM itself**: `Form.TextOut`, `Line`, `Rectangle`,
  `FillRect`, `Circle`, `Pset`, `Pixel`, `Draw`, `TextWidth`, `TextHeight`,
  `Cls`, … — the RapidQ "Hello World" that draws in its form's `OnPaint`
  now shows its text.
  - Same shared bitmap model as QCANVAS, so the desktop, the web, native
    and interpreted builds draw the same pixels.
  - The form gets its surface the first time it's drawn on. It lies under
    the form's controls, the form's own `Color` shows through where
    nothing is drawn, and it takes no mouse events.
  - Text uses the form's `Font` (name, size, color, styles).
  - The surface is the form's client area (`ClientWidth` × `ClientHeight`).
- A form paints again (`OnPaint`) when its size changes: the user drags
  its edge, or the program sets `Width` / `Height`. Sizes stored while the
  form is being declared or laid out don't.
- Tests: `form_surface` (conformance, both backends), `form_draw.bas`
  (desktop, native and interpreted agree) and checks in
  `tests/web_ide_canvas.mjs` (the browser canvas under the controls shows
  the model's pixels).

### Changed
- `Form.Repaint` / `Refresh` don't fire `OnPaint` (a handler that calls
  them would loop); a canvas's do.

## [2.48.0] — 2026-09-28

### Added
- **`OnPaint`** for QCANVAS and QFORM, on desktop and web, natively and
  interpreted. RapidQ programs draw their canvases there, and it was
  never fired, so those programs showed nothing.
  - It fires once when a form is built (the form, then each canvas),
    again when a canvas is resized, and when the program calls the
    canvas's `Repaint`, `Refresh`, `Update` or `Paint` (no arguments).
  - It isn't fired when the window is only uncovered: a canvas keeps
    what's drawn on it, so there is nothing to redraw. A handler that
    draws can't make more `OnPaint` events.
- Tests: `tests/fixtures/canvas_onpaint.bas` (native and interpreted agree)
  and new checks in `tests/web_ide_canvas.mjs`.

### Not yet
- Drawing on a QFORM itself (`Form.TextOut`, `Form.Line`): its `OnPaint`
  fires, but the form has no surface to draw on yet.

## [2.47.0] — 2026-09-28

### Added
- **QCANVAS is drawn by the shared bitmap model**, like QBITMAP and a
  QIMAGE's picture: every drawing method draws into one surface the
  control's size, and each runtime only shows its pixels. The desktop, the
  web, native and interpreted builds now draw the same pixels.
  - `TextOut(x, y, text, color, background)`, `TextWidth`, `TextHeight` and
    `Font` / `Font.*` on a canvas, in the built-in Liberation fonts (as
    on bitmaps since v2.46.0).
  - `Pixel(x, y)` reads a canvas back; `RoundRect`, `Paint(x, y, color,
    border)` (flood fill), `CopyRect` and `StretchDraw` work on canvases
    as they do on bitmaps.
  - The canvas keeps its `Color` as background: `Cls` fills with it, and
    a resized canvas shows it in the new area.
  - `PenColor` / `BrushColor` / `FontColor` / `FontSize` are the colors
    and size drawing uses when none is given.
  - RapidR's own forms are kept: `DrawText`, `Cls`, `Circle(cx, cy, r
    [, color])`, `FillCircle`, `Ellipse`, `SetFont`, `SetPixel`.
- Tests: `canvas_surface` (conformance, both backends) and
  `tests/web_ide_canvas.mjs` (the HTML canvas shows exactly the model's
  pixels).

### Fixed
- **Desktop `Rect` / `FillRect` read their arguments as x, y, width,
  height**; RapidQ's (and the web's) are the two corners, x1, y1, x2, y2.
  Programs that filled rectangles (the corpus's games) drew the wrong size
  on the desktop.
- Compiler warnings on the web build (unused variables in the file list).

## [2.46.0] — 2026-09-28

### Added
- **Text on bitmaps**: `TextOut(x, y, text, color, background)`,
  `TextWidth`, `TextHeight`, `Font` and `Font.*` on QBITMAP, and on a
  QIMAGE's picture.
  - The same pixels on desktop and web, natively and interpreted.
  - It uses the fonts now built into RapidR: the **Liberation fonts** (Sans,
    Serif, Mono; SIL Open Font License 1.1). They have the same character
    widths as Arial, Times New Roman and Courier New, the Windows fonts
    RapidQ programs name, so their text layouts come out the same.
  - A font name with Courier / mono picks Liberation Mono; Times / serif /
    Roman picks Liberation Serif; anything else, Liberation Sans.
  - Sizes are points at 96 dpi, as on Windows screens.
  - Bold, italic, underline and strike-out are drawn from the regular
    faces.
  - The font files are read with `ttf-parser` (MIT OR Apache-2.0) and
    filled by RapidR's own anti-aliased rasterizer.
  - They're credited in `LICENSES.md`, with the license in
    `crates/rapidr-value/fonts/`. The web runtime grows by about 1.1 MB.
  - The corpus's *Print QBitmap* now prints its numbers.

### Tests
- New conformance case `bitmap_text` (widths, heights and the exact
  pixels drawn); 104/104 pass on both backends. New rasterizer unit
  tests.
- The 24 corpus programs that draw or measure text run the same natively
  and interpreted.

## [2.45.0] — 2026-09-28

### Added
- **The PRINTER object**, the same in both backends, from one shared model
  (`rapidr_value::objects::printer`).
  - `BeginDoc`, then the drawing methods (`TextOut`, `Line`, `Rectangle`,
    `FillRect`, `Circle`, `Pset`, `Draw`, `StretchDraw`, `CopyRect`),
    `NewPage`, then `EndDoc` or `Abort`.
  - Properties: `Font` (a QFONT, or `Printer.Font.Size = …`),
    `TextWidth` / `TextHeight`, `PageWidth` / `PageHeight` (A4 at
    300 dpi, swapped by `Orientation`), `PageNumber`, `Printing`,
    `Aborted`, `Copies`, `Title`, `Printers(i)` / `PrintersCount` /
    `PrinterIndex`, `Capabilities.*`.
  - The document becomes a PDF: text in Helvetica (the PDF standard font,
    nothing embedded; widths from its metrics), shapes as vectors,
    bitmaps as images.
  - **Where it goes:** on the desktop, `EndDoc` sends it to the chosen
    printer with CUPS `lp`. With no printer it saves a PDF in the current
    directory. `RAPIDR_PRINT_TO=file-or-directory` saves it instead of
    printing. On the web it opens in the browser for printing.
  - The conformance runner and `tools/corpus_compare.mjs` always set
    `RAPIDR_PRINT_TO`, so tests never print on paper.
- **`PLAYWAV file|resource, options`** on desktop and web.
  - A WAV file or a `$RESOURCE` handle; `SND_SYNC` waits, `SND_ASYNC`
    plays in the background, `SND_LOOP` (8, or the manual's 3) repeats.
  - A new sound replaces the one playing; `PLAYWAV ""` stops it.
  - The sound device is opened once and kept open.
  - `SND_SYNC`, `SND_ASYNC` and `SND_LOOP` are among the built-in
    RAPIDQ.INC constants.

### Tests
- New conformance case `printer` (it aborts, so it never prints);
  102/102 pass on both backends. New model unit test for pages and the
  PDF.
- The corpus's Printer and PLAYWAV programs that build run the same
  natively and interpreted.

## [2.44.0] — 2026-09-28

### Added
- **QFILELISTBOX, on desktop and web**: a list box of a directory's files,
  from the shared list model (`rapidr_value::objects::filelist`).
  - `Directory` starts at the current directory; setting it fires
    OnChange.
  - `Mask` (`*.*`; several separated by `;`, case-insensitive as on
    Windows), `AddFileTypes` / `DelFileTypes` (ftDirectory lists `[..]`
    and `[dir]` entries, ftHidden adds dot files), `Update`, `FileName`
    and `Drive`.
  - The list box's own properties and events work as on a QLISTBOX.
- **QDIRTREE, on desktop and web**: a directory tree from the filesystem
  root, shown as indented rows (`rapidr_value::objects::dirtree`).
  - `InitialDir` / `Directory` select a directory, open the ones above it
    and scroll it into view.
  - A click selects a directory (OnChange); a double click opens or closes
    it.
  - `FullCollapse`, `FullExpand` (up to 5,000 rows), `Reload`.
  - Directories are read only when they're opened.
- The web has no filesystem, so there both show nothing, or just the root.

### Fixed
- **`MID$(s, i)` without a length** returns the rest of the string, as in
  QBasic; it returned "".

### Tooling
- `tools/corpus_compare.mjs` removes each program's own native build
  outputs after running it and builds without incremental caches. A full
  run had grown `tests/conformance/.work` to 165 GB and filled the disk.
  Window captures are written next to each program's folder, not in it,
  so programs that list their directory see the same files in both runs.
- Full comparison: the programs that differ from one run to the next do so
  from timing and focus (windows on a live desktop), not from the
  backends; reruns match.

### Tests
- New conformance case `file_list_box`, and two-argument `MID$` in
  `strings`; 100/100 pass on both backends.
- New GUI fixture `file_browser.bas` (a QDirTree driving a QFileListBox)
  in `tests/native_gui_events.mjs`.
- Model unit tests for wildcards, listing and the tree.

## [2.43.0] — 2026-09-28

### Added
- **QSTRINGGRID range selection (goRangeSelect)**, on desktop and web.
  - Dragging over cells, shift-clicking or shift+arrow keys select a block
    of cells.
  - The block is highlighted, and OnDrawCell's `State` has gdSelected for
    each of its cells (gdFocused for the current one).
  - It is on by default, as in Delphi, and off with goEditing, as the
    manual says.
- **gcsList columns** (`ColumnStyle(i) = gcsList`, `ColumnList(i)`). The
  selected cell shows a drop-down button listing the column's items.
  Picking one stores it like an edit: the cell, then OnSetEditText, then
  OnChange.
- **goColSizing / goRowSizing**: dragging a header's border resizes the
  column or row, and `ColWidths` / `RowHeights` follow.

### Fixed
- **Selecting the current cell again clears a range selection.** OnDrawCell
  also fires again when the selection changes, with each cell's new
  `State`, as Delphi redraws selected cells.

### Tests
- New fixture `grid_range_list.bas`, in `tests/native_gui_events.mjs` and
  `tests/web_ide_grid_draw.mjs`. The browser test drags a range, picks
  from a drop-down and resizes a column. New model unit test.
- The corpus's grid and list-view programs (9) run the same natively and
  interpreted.

## [2.42.0] — 2026-09-27

### Added
- **QSTRINGGRID `OnDrawCell(Col, Row, State, Rect, Sender)`, on desktop
  and web from the shared grid model.**
  - After the grid's content, sizes, selection or options change, or on
    `Repaint` / `Refresh`, the event fires once for every cell (up to
    20,000).
  - `State` uses Delphi's bits (gdSelected = 1, gdFocused = 2,
    gdFixed = 4). `Rect` is a QRECT in the grid's coordinates.
  - What the handler draws on the grid is kept on its cell and drawn over
    it: `Line`, `Rectangle`, `FillRect`, `Circle`, `Pset`,
    `TextOut(x, y, text, color, background)` and `Draw(x, y, bitmap)`.
  - Reading the grid in the handler doesn't fire the event again.
  - The corpus's `chkgrid.bas` draws its checkbox column, identically
    natively and interpreted.

### Changed
- **Every event passes its component last, as `Sender`**, as in RapidQ
  (`SUB DrawCell (…, Rect AS QRECT, Sender AS QSTRINGGRID)`). Before, only
  events without arguments passed it. Handlers that declare fewer
  parameters are unaffected.
- **Grid cell text is drawn at Left + 2, Top + 2**, as Delphi's grid draws
  it, on desktop and web. It was vertically centered.

### Tests
- `tests/fixtures/grid_draw_cell.bas` in `tests/native_gui_events.mjs`
  (native and interpreted) and in the new `tests/web_ide_grid_draw.mjs`;
  new model unit test.

## [2.41.0] — 2026-09-27

### Added
- **QCOMBOBOX edit box, as in RapidQ.** The default style, csDropDown,
  lets the user type as well as pick, on desktop and web.
  - Typing sets `Text`, and `ItemIndex` becomes the matching item or -1,
    before OnChange fires.
  - `Style = csDropDownList` (2) keeps a pick-only list.
  - RapidR's own examples that use combos as pick lists now set it.

### Fixed
- **QLISTBOX with `MultiSelect` on the desktop** now shows every selected
  item, and the user can select several (click, shift-click). The
  selection goes into `Selected(i)` / `SelCount`, with `ItemIndex` as the
  clicked item. Before, only one item was ever shown selected.

### Tests
- `tests/web_ide_lists.mjs` types into the combo; new unit test for the
  user's multi-selection.

## [2.40.0] — 2026-09-27

### Added
- **`$RESOURCE`: files built into the program, on both backends.**
  - `$RESOURCE NAME AS "file"` makes `NAME` the resource's handle.
    Several directives may share a line, separated by `:`.
  - The files are built into the program: interpreted builds carry them
    in a new optional bytecode section; native builds use `include_bytes!`.
  - A missing file is a compile error, as in RapidQ.
  - New: `RESOURCE(n)`, `RESOURCECOUNT`, `EXTRACTRESOURCE handle, file`
    and `Stream.ExtractRes(handle)`.
  - `BMPHandle = NAME` and `ImageList.AddBMPHandle NAME` show a resource
    bitmap.
- **QIMAGE shows and draws pictures, the same on desktop and web.**
  - It is backed by the shared bitmap model (`rapidr_value::objects`),
    like QSTRINGGRID and QLISTBOX.
  - Supported: `BMP`, `BMPHandle`, `AutoSize`, `Stretch`, `Center`,
    `Transparent` (the bottom-left pixel's color, as in Delphi's TImage),
    `Pixel`, and the QBITMAP drawing methods.
  - Drawing on an image without a picture first gives it one the size of
    the control.
  - PNG and JPEG files still load as before.
  - The RapidQ Othello game now shows its board and chips, identically
    both ways.
- **QIMAGE mouse events** in RapidQ's order, on desktop and web:
  OnMouseDown / OnMouseUp (Button, X, Y, Shift), OnMouseMove (X, Y,
  Shift), OnClick, OnDblClick.
- **`MOUSEX` / `MOUSEY`**: the mouse position relative to the active
  form's client area.

### Fixed
- **Native builds: a typed main-program variable read inside a CREATE
  block** (`Width = bx`) read an empty value (since v2.38.0). The
  Sokoban level editor's board was invisible because of it.
- **`Circle(x1, y1, x2, y2, c, fill)`** fills with the color `fill`, as
  in RapidQ; it was treated as a yes/no flag and filled with `c`.
- **Interpreter: `True` / `False` without RAPIDQ.INC** are -1 / 0, as in
  native builds; they were empty.
- **Native builds: reading `Img.Center`** reads the property; it called
  the Center method.
- **Native builds: a component DIMmed inside a block** (`IF … DIM Dlg AS
  QOPENDIALOG`) was never created, so RapidQ's rqb2html skipped its file
  dialog. Components are now created by their DIM at any depth, as in the
  interpreter.

### Tests
- **Corpus comparison** (`tools/corpus_compare.mjs`): 98 of the 101
  programs that build run the same natively and interpreted. The other 3
  use random numbers or the clock. `graphics/rotate/rotate.bas` no longer
  builds, because its `$RESOURCE` file isn't in the corpus.
- New conformance cases `resources` and `nested_component_dim`;
  `typed_globals` covers CREATE blocks; 98/98 pass on both backends.
- `tests/fixtures/picture_resource.bas` in `tests/native_gui_events.mjs`,
  natively and interpreted.
- New `tests/web_ide_picture.mjs` for the browser.

## [2.39.0] — 2026-09-27

### Added
- **QMEMORYSTREAM / QFILESTREAM `SaveArray` and `LoadArray`**, on both
  backends from one shared rewrite (`rapidr_ast::stream_arrays`).
  - `Stream.SaveArray(A(i), n)` writes `n` elements starting at `A(i)`;
    `LoadArray` reads them back.
  - Each element takes its declared type's bytes, as in RapidQ: BYTE 1,
    SHORT 2, LONG 4, SINGLE 4, DOUBLE 8, …
  - For a multi-dimensional array the last index steps, so
    `LoadArray(vertex(i, j, 0), 3)` fills `vertex(i, j, 0..2)`.
  - Loading or saving stops at the array's last element. RapidQ wrote past
    it; the corpus's own `arrins.bas` does this.
  - The corpus programs `arrins.bas` and `sieve.bas` now run, and run the
    same both ways.

### Fixed
- **`Stream.Read(x)` and `Stream.Write(x)` use `x`'s declared size.**
  Before, BYTE, SHORT, WORD, DWORD and SINGLE variables always took 4 or
  8 bytes.

### Tests
- New conformance case `stream_arrays`; 94/94 pass on both backends.

## [2.38.0] — 2026-09-27

### Added
- **The RapidQ corpus now runs the same natively and interpreted.**
  `tools/corpus_compare.mjs` builds every program that compiles, both
  ways, runs each (no input, a time limit, window captures), and compares
  what they print and show.
  - 99 of the 102 programs it runs look and print the same. The other 3
    use random numbers or the clock, so their differences are expected.
  - 14 network programs are skipped, so the tool never contacts real hosts.
  - The run found and led to the fixes listed below.
- **Native builds: typed main-program variables and parameters.**
  - A numeric variable of the main program (`DIM total AS LONG`, a FOR
    counter, …) is now a Rust number in an atomic static rather than a
    boxed `Value` slot. SUBs that don't declare their own variable of that
    name use it too.
  - A BYVAL numeric parameter is a Rust number inside its SUB/FUNCTION.
  - The same exclusions as typed locals apply: nothing that could store
    something else into the variable (BYREF, `@`, array or object use,
    fractional STEP, inline Rust).
  - Benchmark, 5M iterations of a main-program loop: 0.19 s → 0.01 s
    native; 0.56 s interpreted.
- **Run-time errors in the interpreter name the file and line:** `…
  (at QFlatButton.inc line 12)`. The bytecode carries a small source map,
  with file names only (never full paths), as an optional section that
  older readers ignore.

### Fixed
- **Desktop, both backends: a crash when a form was resized while being
  shown** (the window manager adjusting it). The resize callback ran while
  the runtime held its widget table ("RefCell already mutably borrowed").
  Introduced in v2.35.0.
- **Native: `Font = Font` inside a CREATE block was dropped** when a QFONT
  variable had that name. It now sets the created control's font from the
  variable, as in RapidQ and the interpreter.
- **Interpreter: `IF Form.ShowModal THEN` / `IF OpenDialog.Execute THEN`**
  (the method called without parentheses for its result) read a property
  instead of showing the form or dialog. Both backends now share the list
  (`rapidr_ast::VALUE_METHODS`).
- **Both backends: `DIM r AS QRegistry` (a RapidQ object RapidR doesn't
  implement) inside a SUB, FUNCTION or TYPE method.** The interpreter
  stopped with "not an object" while native builds went on. Now the local
  refers to an object of its name, as in the main program: its methods
  warn and the program continues (`rapidr_ast::routine_objects`).
- **Security: bytecode decoding checks every count** against the bytes
  left, so a corrupt `.rrbc` can't make the reader allocate gigabytes.
  Slice bounds are computed with checked arithmetic (a 32-bit overflow on
  WebAssembly could bypass the check), and a function's local count is
  limited to the 65,536 slots the instructions can address.

### Tests
- Conformance case `typed_globals`: typed main-program variables used in
  SUBs and shadowed by locals, typed BYVAL parameters (reassigned), BYTE
  wrap, GOTO / GOSUB with a typed FOR counter, a global passed BYREF,
  DIM reset, and INPUT into a typed variable. 92/92 on both backends.
- Unit tests: bytecode source map round trip (names only), corrupt counts
  rejected, and code-generation tests for typed globals.

## [2.37.0] — 2026-09-27

### Changed
- **A form's Width / Height are its whole window, and ClientWidth /
  ClientHeight its inside**, as in RapidQ (Delphi), and the same on the
  desktop and the web (`rapidr_value::layout::form_client_size`).
  - The frame is a 29px caption and a 1px border on every side, plus the
    main menu. A 400 × 300 form has a 398 × 269 client area without a menu.
  - `BorderStyle = bsNone` (0) removes the frame: the window has no
    decorations on the desktop and no title bar or border on the web.
  - Setting `ClientWidth` / `ClientHeight` sets the size that gives that
    inside.
  - Before, a desktop form's Height was its inside (31px taller than in
    RapidQ). A web form's Width left out its border, and `BorderStyle = 2`
    (bsSizeable, the default) gave the web form a dashed border.
  - Other controls' BorderStyle on the web is now none (0) or a solid line.

### Added
- **QSPLITTER can be dragged**, on the desktop and the web, as Delphi's
  TSplitter (`rapidr_value::layout::splitter_drag`):
  - it resizes the control just outside its anchored edge (left of an
    alLeft splitter, above an alTop one, …);
  - it never goes below `MinSize` (30) or leaves less than `MinSize` for
    the rest of the client area;
  - the other aligned controls re-flow as it moves, and `OnMoved` fires
    when the drag ends;
  - the cursor shows the direction.
- Test hook `RAPIDR_TEST_SPLIT=splitter:delta` drags a splitter as the
  mouse would. `RAPIDR_TEST_RESIZE` now takes the form's Width,Height.

### Tests
- `align_layout` (native and interpreted agree) and `tests/web_ide_align.mjs`
  now check the client area of a framed form and a splitter drag with
  OnMoved. The browser test drags with a real mouse. Both runtimes give
  the same numbers for the same program.
- Unit tests for the frame sizes and for splitter drags (neighbour found,
  MinSize, room left, alBottom direction, nothing to resize).

## [2.36.0] — 2026-09-27

### Fixed
- **QLISTBOX and QCOMBOBOX work like RapidQ's.** Both runtimes now draw
  one shared model, `rapidr_value::objects::list`. The RapidQ corpus uses
  `Item` 294 times, `AddItems` 143 and `ItemCount` 90.
  - **Desktop:**
    - `AddItems a, b, c` kept only the last item;
    - `ItemCount` and `Item(i)` weren't implemented;
    - a list box selection didn't fire `OnDblClick`.
  - **Both runtimes:**
    - `Item(i)` read / `Item(i) = s`, `ItemCount`, `ItemIndex` (−1 for
      none), `AddItems`, `InsertItem i, s`, `DelItems i, …` (indexes as
      before any deletion), `Clear`;
    - `Sorted` (case-insensitive; the selected item stays selected),
      `MultiSelect` with `Selected(i)` and `SelCount` (setting `Selected`
      keeps ItemIndex, as in Delphi), `TopIndex`;
    - `Text`: a combo box's current text, which follows ItemIndex and
      selects the matching item when set; a list box's items as
      CRLF-terminated lines;
    - `LoadFromFile` / `SaveToFile`, one item per line.
  - The user's pick is stored before the program's `OnClick` / `OnChange`
    runs, so handlers read the new ItemIndex / Text.
  - **Items are shown exactly as written:** FLTK's `@` formatting codes
    and a menu's `/`, `&` and `\` are escaped on the desktop, and the web
    uses plain-text options.
  - The web list box shows rows rather than a drop-down; its options are
    redrawn once per batch of changes.
- RapidR's older names still work on the same model: `AddItem`,
  `DeleteItem` / `RemoveItem`, `Items`, `Count` / `ListCount`,
  `ListIndex`, `Find`.

### Tests
- `tests/fixtures/list_items.bas`, checked natively and interpreted
  (`tests/native_gui_events.mjs`, both agree) and in the browser
  (`tests/web_ide_lists.mjs`, including picking items with the mouse).
- Unit tests for the list model: every item kept, index tracking through
  inserts and deletes, sorting, multi-select, combo text, legacy names,
  out-of-range indexes.

## [2.35.0] — 2026-09-27

### Added
- **`Align` works on the desktop and the web**: alTop, alBottom, alLeft,
  alRight and alClient, laid out as RapidQ's Delphi VCL controls are.
  52 of the corpus's example programs use it.
  - The placement is one shared function, `rapidr_value::layout`, with
    Delphi's `AlignControls` rules:
    - order alTop, alBottom, alLeft, alRight, then alClient;
    - each aligned control keeps its height (top/bottom) or width
      (left/right);
    - controls with the same Align are ordered by position, and the one
      whose Align, size or visibility just changed goes first (so a
      splitter created before its tree ends up to the right of it);
    - invisible controls are skipped.
  - Layout works on the stored Left / Top / Width / Height, so a program
    reads the aligned size immediately (`Grid.Align = alClient : PRINT
    Grid.Width`), before or after the form is shown.
  - A container is laid out again when:
    - an aligned child changes Align, position, size or visibility;
    - the container itself is resized;
    - a form gets its main menu, which takes the top of the client area.
  - QSTATUSBAR is alBottom and QSPLITTER alLeft by default, as in RapidQ.
    The status bar now takes its space from the client area instead of
    covering the controls behind it (web) or being drawn over them
    (desktop).
- **Desktop: live geometry.** Setting Left / Top / Width / Height on a
  control that is already shown moves or resizes it; before, only the
  stored value changed. `Form.Left` / `Form.Top` move the window, and
  `Center` records where the form went.
- **Desktop: resizing a form by hand** updates Width / Height, lays out its
  aligned controls and fires `OnResize`. Other controls keep their places,
  as in RapidQ; before, FLTK scaled every control in proportion.
- **Web:** maximizing or restoring a form lays it out again and fires
  `OnResize`; dragging a form updates its Left / Top.
- `ClientHeight` is the form's height less its in-window main menu (on the
  desktop, except on macOS where the menu is the system menu bar).

### Fixed
- **Panels show their Caption, centered.** On the web, setting a panel's
  Caption no longer deletes the controls inside it.
- **Web geometry reads:** Left / Top read what the program or the layout
  set; Width / Height fall back to the stored value while an element isn't
  rendered. Before, a control on a form not yet shown read 0.
- **Web defaults match the desktop:** the splitter is 5px wide and the
  status bar 24px high.
- **Test hooks:** captures (`RAPIDR_CAPTURE`) draw what event handlers just
  changed; interpreted programs' captures could show the previous state.

### Tests
- `tests/fixtures/align_layout.bas` in `tests/native_gui_events.mjs`: every
  Align, the splitter's default, hiding a control, and a form resized as by
  the user. The new `RAPIDR_TEST_RESIZE=w,h` hook drives the resize.
  Native and interpreted builds agree.
- `tests/web_ide_align.mjs`: the same program in the browser, including
  element positions, maximize with OnResize, and a panel's caption keeping
  its children.
- Unit tests for the layout function (edges, ordering, invisible
  controls, running out of space).

## [2.34.0] — 2026-09-26

### Added
- **QSTRINGGRID works like RapidQ's, on the desktop and the web.** Both
  runtimes draw one shared model (`rapidr_value::objects::grid`), so a
  grid behaves the same natively, interpreted, and in the browser.
  - `Cell(col, row)` / `Cells` read and write, `ColCount` / `RowCount`
    (5 each by default), `FixedCols` / `FixedRows` (1 each),
    `DefaultColWidth` (64) / `DefaultRowHeight` (24), `ColWidths(i)`,
    `RowHeights(i)`, `Col`, `Row`, `TopRow`, `LeftCol`, `GridWidth`,
    `GridHeight`, `EditorMode`.
  - `InsertRow`, `DeleteRow`, `InsertCol`, `DeleteCol`, `SwapRows`,
    `SwapCols`, `AddOptions`, `DelOptions`, `ColumnStyle(i)`,
    `ColumnList(i)`.
  - `SaveToFile` / `LoadFromFile` / `SaveToStream` / `LoadFromStream`
    `(file or stream, RowOffset, ColOffset, MaxRows)`: rows of cells joined
    by `Separator`.
  - **Desktop:** a real table widget (FLTK `Table`), not rows of text
    boxes. It has a shaded fixed header row and column, per-column widths
    and per-row heights, scrolling, and a highlighted selected cell.
  - **Web:** a table redrawn once per batch of changes.
  - **Interaction, both runtimes:**
    - clicking or the arrow keys select a cell (fixed cells can't be
      selected) and fire `OnSelectCell(Col, Row, CanSelect)` and `OnClick`;
    - with `goEditing` (`AddOptions 10`), a double-click, Enter, F2 or
      typing edits the cell in place (a single click with
      `goAlwaysShowEditor`). Enter or leaving the cell stores it and fires
      `OnSetEditText(Col, Row, Value$)` and `OnChange`; Escape cancels;
    - an ellipsis column (`ColumnStyle(i) = gcsEllipsis`) shows a button
      that fires `OnEllipsisClick(Col, Row)`.
  - Sizes are capped (at most 4 million cells), and cell text is always
    drawn as plain text.
- RapidR's own grid API is kept on the same model: `AddRow a, b, …`
  (widens the grid if needed), `SetCell` / `GetCell`, `Clear` (no rows),
  `SetRowCount` / `SetColCount`, `Cols` / `Rows` / `ColWidth`,
  `SelectedRow` / `SelectedCol`, and a "..." cell showing an ellipsis
  button.

### Changed
- **`SetCell` / `GetCell` take `(col, row)` in both runtimes**, the same
  order as RapidQ's `Cell`. The desktop runtime used `(row, col)` and the
  web runtime `(col, row)`.
- A grid starts at RapidQ's 5 × 5 with one fixed row and column, and cells
  can be edited only with `goEditing`. RapidR's IDE programs
  (`examples/ide.rr` and others) now set `RowCount = 0` before filling
  grids with `AddRow`, and `AddOptions 10, 13` where cells are edited.
- `RDataFrame.ToGrid` fills the grid through the same model on the
  desktop.

### Tests
- `tests/web_ide_grid.mjs`: cells, InsertRow/SwapRows, widths, fixed
  shading, plain-text cells, selection by click and arrow keys, in-place
  editing with `OnSetEditText`, the ellipsis button, `AddRow` / `SetCell` /
  `GetCell`.
- `tests/fixtures/string_grid.bas` in `tests/native_gui_events.mjs`: rows
  and columns, `Separator`-based streams and selection; native and
  interpreted builds agree.
- Unit tests for the grid model: defaults, row/column edits, the text
  round trip, out-of-range indexes and huge sizes.

## [2.33.0] — 2026-09-26

### Changed
- **Every RapidQ example the interpreter accepts now compiles natively.**
  Of the 116 corpus programs that compile to bytecode, 116 pass `cargo
  check` as native builds, up from 49. The native compiler now resolves
  what the interpreter already did, the same way:
  - methods, properties and indexed properties of objects that aren't
    component variables: `Application.Terminate`, `Screen.Cursors(i) = h`,
    `Printer.Printers(i)`, `RichEdit.SelAttributes.Color = c`,
    `This.Grid.Cell(x, y) = s`, `DXTimer.OnTimer = Handler`;
  - these objects are resolved by name in the main program, and by the
    component id a SUB's own parameter or local holds (the VM's rule);
  - `Cell(1, 0) = s` / `ColWidths(0) = w` inside a CREATE block set an
    indexed property of the object being created;
  - WITH blocks use the shared `rapidr_ast::resolve_with_body`, so
    `.Member` works on every kind of object, in statements and expressions;
  - variables named like builtins (`RGB$ = …`), arrays declared with a type
    suffix (`DEFSTR Rider$(1 TO 29)`), FUNCTIONs named with a suffix
    (`QSystem.OSName$`), and dotted names with a suffix in the middle;
  - a SUB's name used as a value is empty, and an event handler the program
    never defines binds nothing;
  - `BIND ptr TO Prototype` with no such routine only gives the pointer a
    signature;
  - a call with more arguments than the routine has parameters drops the
    extra ones (all are still evaluated, in order); missing ones are empty;
  - a routine nobody defines, which only unreached library code can call,
    is a run-time error, as in the VM;
  - program files whose names aren't valid Cargo package names
    (`Cancel Form Close.bas`, `3dview.bas`) build; the executable keeps the
    file's name.

### Fixed
- **Both backends.**
  - A number written with a leading dot (`SetRGBA(.1, 1, .1, .7)`) was
    read as a WITH member.
  - A TYPE method named `Init` (or `Ctor`, `Ev0`) replaced the routine that
    sets up the TYPE's instances. The generated routines are now
    `Type___init`, `Type___ctor` and `Type___ev<i>`.
  - SUBs and FUNCTIONs written inside another SUB are hoisted to the top
    level (`rapidr_ast::hoist_routines`); native builds couldn't bind them
    as event handlers.
  - `CURDIR$`, `RND` and `DIR$` work without parentheses, unless a variable
    has that name.
- **Interpreter.**
  - `REDIM a(…)` inside a SUB made a new local array instead of resizing
    the module-level one. It now resizes it, unless the SUB declares its
    own `a`.
  - `Screen.Cursors(i) = h` and `Grid.Cell(x, y) = s` on objects that
    aren't components failed or did nothing; they call the object's
    method, as native builds do.

### Tests
- Conformance case `native_parity`: leading-dot numbers, a builtin-named
  variable, suffixed arrays, REDIM of a module array in a SUB, nested SUBs,
  a TYPE method named `Init`, argument fitting, and a suffixed FUNCTION
  called without its suffix.
- `tools/corpus_native.sh <corpus.json>`: the native corpus check (was a
  scratch script).

## [2.32.0] — 2026-09-26

### Changed
- **Native builds keep numeric locals as Rust numbers.** A SUB/FUNCTION's
  local declared BYTE, WORD, SHORT, INTEGER/LONG, DWORD, SINGLE or DOUBLE is
  an `i64` / `f64` instead of a boxed `Value`. Arithmetic, comparisons,
  AND/OR/XOR/NOT, IF/WHILE/DO conditions and FOR loops over such variables
  compile to plain Rust, with exactly `Value`'s semantics:
  - wrapping integer `+ - *`;
  - `/`, `\` and MOD by zero give 0;
  - comparisons are done as floats, with NaN treated as equal;
  - FOR evaluates end and step once.

  A local stays a `Value` when something else could store into it: it's
  passed to a user SUB/FUNCTION (a possible BYREF), its address is taken,
  it's used as an array or object, an integer counter has a fractional
  STEP, or the routine uses GOTO/GOSUB or inline Rust.
  - Benchmark (5M iterations of `MOD`, `/`, `AND` and IF in a SUB): 0.02 s
    typed, 0.13 s as `Value`s, 0.62 s interpreted; all three print the same
    result.
  - On the RapidQ example corpus, the same 49 programs `cargo check`
    natively before and after, so typed locals introduced no failures.

### Fixed
- A FOR counter's start value converts to the counter's declared type in
  both backends (`FOR i = 1.5 TO 4` with `i AS INTEGER` starts at 2).

### Tests
- Conformance case `typed_locals`: every operator on typed LONG / DOUBLE /
  SHORT / BYTE locals, including division and MOD by zero, NaN comparisons,
  overflow, negative and fractional STEPs, DIM re-run in a loop, WHILE/DO,
  a BYREF-passed local, and a typed FUNCTION.
- Unit tests pin `rapidr_value::numeric`'s typed helpers to `Value`'s
  operators on edge values.

## [2.31.0] — 2026-09-26

### Changed
- **Declared numeric types are enforced, in both backends** (RapidQ manual,
  Appendix C: a type "cannot be changed once bound"). Storing into
  something declared BYTE, WORD, SHORT, INTEGER/LONG, DWORD, SINGLE or
  DOUBLE now converts the value; before, `DIM n AS INTEGER : n = 2.5` kept
  2.5 and a BYTE held 300.
  - Integers round half to even, the rounding RapidQ documents for ROUND:
    2.5 → 2, 3.5 → 4, -2.5 → -2.
  - Integers wrap to their width: BYTE 0..255, WORD 0..65535, SHORT 16-bit,
    INTEGER/LONG 32-bit, DWORD 32-bit unsigned.
  - SINGLE and DOUBLE hold floating-point numbers; a string converts like
    VAL.
  - This applies to assignments, `INPUT`, array elements, TYPE fields
    (including array fields), BYVAL parameters on entry, and a typed
    FUNCTION's result (`f = …`, `RESULT = …`, `RETURN …`).
  - One shared pass (`rapidr_ast::numeric`) inserts the conversions; the
    rules live in `rapidr_value::numeric`. The VM runs each conversion as a
    single opcode (`ToNum`); native builds call it directly.
  - Not yet: FOR counters' own increments, and type suffixes (`n%`, `n&`).

### Fixed
- **Native builds:** a SUB/FUNCTION's own `DIM x` or parameter `x` now
  shadows a module-level `x`, as in the interpreter. Native code used to
  read and write the global instead (`DIM n AS STRING` in a SUB overwrote
  the program's `n`).

### Tests
- Conformance case `numeric_types` (both backends): rounding, wrapping of
  every type, strings, arrays, TYPE fields, parameters, FUNCTION results,
  globals changed in a SUB, local and parameter shadowing, and `INPUT`.

## [2.30.0] — 2026-09-26

### Security
- **The interpreter is sound when events fire while it runs** (SEC-08).
  Both VM hosts used to reach the VM through a raw pointer from event
  callbacks. An event fired while the VM was inside a host call (an
  `OnClose` fired by `Form.Close`, a button click during `ShowModal`, a
  timer) created a second `&mut Vm` alongside the running one. That is
  undefined behaviour in Rust, and an `OnClose` handler that opened a
  dialog could leave the rest of the click handler running on the wrong
  frames. Now:
  - the runtimes only **queue** the handlers they fire; the VM runs them
    itself at safe points, right after the operation that fired them, each
    to completion before the next (`Host::take_events`);
  - the desktop's `ShowModal` and the program's windows are a wait the VM
    serves (`Host::wait_started` / `Host::pump`): it pumps UI events and
    runs their handlers between them, so nested modal forms work;
  - the browser keeps the program in a `RefCell` session, and JavaScript
    callbacks run queued handlers only while the VM is idle;
  - an event handler runs on top of the code it interrupted, with one
    frame stack. A handler that waits for a dialog, or pauses in the
    debugger, finishes on resume, and then that code continues;
  - `rapidr-vm`, `rapidr-vm-host-native` and `rapidr-vm-host-web` now
    `#![forbid(unsafe_code)]`.

  Speed is unchanged: 3M builtin calls take 1.44 s, against 1.42 s before.

### Changed
- `END` inside an event handler ends the program, including the code the
  handler interrupted (as in a native build).
- A run-time error in a handler fired during a statement stops the
  program, as it does natively. Handlers of UI events that fail are still
  reported and the program goes on.
- Event handlers fired inside another handler (e.g. `OnClose` from
  `Form.Close`) can now use the in-page dialogs; they used to fall back to
  the browser's `alert`/`confirm`.

### Fixed
- The web debugger's variables and stack trace JSON escapes names.
- Each IDE run replaces the previous program's session instead of leaking
  it.

### Tests
- VM unit tests for queued events, their order, a handler that waits for a
  dialog, a host-driven wait, and `END` inside a handler.
- `tests/fixtures/nested_modal.bas`: timers during `ShowModal`, plus a
  modal form opened and closed by a timer inside a handler, checked
  natively and interpreted.
- `tests/web_ide_reentrant_events.mjs`: `OnClose` fired inside a click
  handler shows a dialog, then both handlers finish in order. This test
  fails on v2.29.0.

## [2.29.0] — 2026-09-26

### Added
- **QSTATUSBAR panels.** `AddPanels "Ready", "Line 1"`, `Panel(i).Caption`,
  `Panel(i).Width` (100 by default; the last panel takes the rest),
  `PanelCount`, `SimplePanel` and `SimpleText` are drawn on the desktop and
  in the web runtime, and update when the program changes them.
- **QLISTVIEW columns, items and sub-items** (RapidQ manual, Appendix A),
  one data model shared by both runtimes (`rapidr_value::objects::listview`):
  - `AddColumns`, `ClearColumns`, `Column(i).Caption` / `.Width`,
    `ColumnsCount`;
  - `AddItems`, `InsertItem`, `DelItems`, `Clear`, `SwapItem`,
    `Item(i).Caption` / `.Checked` / `.Selected` / `.ImageIndex` / `.Index`,
    `ItemCount`, `ItemIndex`, `SelCount`;
  - `AddSubItem`, `InsertSubItem`, `DelSubItem`, `SubItem(i, j)`;
  - clicking a row sets `ItemIndex` and fires `OnClick` / `OnDblClick`;
    clicking a header fires `OnColumnClick(Column%)`.
  Captions are always shown as plain text (no FLTK `@` codes or HTML
  markup), and out-of-range indexes are harmless.
- `ClientWidth` / `ClientHeight` on forms and containers (RapidQ's ZIP
  viewer example sized its list view with them and showed nothing).

### Changed
- **`rapidr build --interp` makes optimized executables by default.** It
  used the debug interpreter unless `--release` was given: 328 MB and far
  slower. The runner now comes from a stripped `runner` profile, so the
  ZIP viewer is 44 MB. `--debug` still selects the debug runner. The runner
  is built once; later builds take about 0.2 s.

### Fixed
- `rapidr build --interp` (and native builds) now work from any directory:
  the CLI finds the RapidR workspace from `RAPIDR_HOME`, the current
  directory, its own executable, or where it was compiled. It used to fail
  with "could not find `Cargo.toml`" outside the repository.

## [2.28.1] — 2026-09-26

### Removed
- **Old object code in both compilers (about 800 lines).** Since v2.27.0
  every TYPE goes through the shared object front end
  (`rapidr_ast::objects`) before either backend sees it, so the interpreter
  compiler's own TYPE handling (instance setup, method lookup, PROPERTY SET
  setters, EVENT trampolines, `This` aliases) and the native code
  generator's UDT-struct path could no longer run. Both are gone; the
  compilers keep only their component paths (`Sender AS QBUTTON`, arrays of
  components, indexed sub-objects). No behaviour change: conformance 84/84
  on both backends, the RapidQ example corpus stays at 116/386, 44/44 native
  examples, GUI and web IDE suites pass.

## [2.28.0] — 2026-09-26

### Security
- **Hostile values no longer crash the runtime.** Integer `\`, `MOD` and
  unary minus wrap instead of panicking (`MIN \ -1` took the whole program
  down), and `INV` computes in 128 bits. The fix is in the shared value
  layer, so both backends have it.
- **Memory can't be exhausted by one call:**
  - strings are capped at 256 Mi characters; `SPACE$(1E12)`, `STRING$` and
    a string doubled in a loop stop with a clear run-time error instead of
    hanging while allocating;
  - the interpreter stops runaway recursion at 100,000 nested calls with a
    "stack overflow" error instead of growing without bound (native builds
    already stop safely with Rust's stack-overflow abort).
- **Fuzzing:**
  - every builtin was called with edge arguments in the interpreter (4,617
    calls; the one crash found, `INV`, is fixed);
  - 1,344 mutated RapidQ programs went through the compiler and the native
    code generator with no crashes.
- **Known issue (roadmap):** the web interpreter host reaches the VM
  through raw pointers, and an event fired from inside a running
  statement re-enters it. This works on single-threaded wasm but isn't
  sound Rust; it needs an event queue or a restructured host. (Fixed in
  2.30.0.)

### Changed
- **Faster module-level variables, in both backends.** They live in slots
  instead of a string-keyed map. The interpreter indexes by the name's
  string-table entry; native code gets `gv(3)` instead of `gv("total")`, so
  every write no longer allocates a key.
- **Faster interpreter core:**
  - array accesses don't allocate an index vector;
  - SUB/FUNCTION calls reuse their locals' storage;
  - the opcode decoders are inlined.
- **Benchmarks** (plain loop: arithmetic, a FUNCTION, an array, 5 million
  iterations): the interpreter went from 2.05 s to 0.97 s, native from
  1.22 s to 0.48 s. The object benchmark (10 million method and field
  operations) now takes 1.21 s interpreted (14.5 s in v2.26.0) and 0.66 s
  native (13.0 s in v2.26.0).

### Fixed
- **`STRING$(n, code)`** repeats the character with that code
  (`STRING$(3, 65)` = "AAA"); it took the first digit. `SPACE$` and
  `STRING$` are shared now (`rapidr_value::strings`).
- **Tests:** conformance case `edge_values`.

## [2.27.0] — 2026-09-26

### Changed
- **Objects are real values with direct field access, in both backends.**
  An instance of a TYPE is now `Value::Object`: shared by reference, with
  its fields in slots fixed at compile time, the ancestors' fields first.
  Before, fields were looked up by name in the runtime's component
  registry.
  - Native builds compile a field access to a direct vector index
    (`obj_field(&c, 1)`); the interpreter has matching `GetField`/`SetField`
    opcodes.
  - On an object benchmark (a method call plus field reads and writes, 10
    million times): the interpreter went from 14.5 s to 2.6 s (5.6×), and
    native builds from 13.0 s to 1.4 s (9.2×).
- **One object front end for both backends: `rapidr_ast::objects`.** The
  bytecode compiler and the Rust generator run the same lowering pass;
  objects become plain routines plus a few builtins that each backend
  implements. The native compiler's own object pass is gone; the
  interpreter's old object code in its compiler no longer runs and will be
  removed in a later cleanup. The two can't drift apart any more.
- **Every TYPE is an object type now.** Plain UDTs (fields only) used to be
  by-value Rust structs in native builds but shared references in the
  interpreter; both use the interpreter's reference semantics now.
- **A method's code is reachable only when it's called** (or its address
  is taken). Before, declaring an instance made every method of its TYPE
  reachable, so an unused method's DLL call could stop an interpreted
  build.

### Added
- **Arrays of TYPE objects:** `DIM a(1 TO 3) AS TType` creates and sets up
  one instance per element (ids `a(1)`, …), constructors included. The
  same holds for array fields of objects, e.g. `Parts(2) AS TPart`.
- **Self-referencing fields start empty.** A field whose TYPE leads back to
  its owner (`Link AS TItem`) starts as Nothing and is assigned by the
  program, which allows linked structures; it no longer recurses forever.
- **EVENT handlers bind to each instance at run time.** This covers array
  elements and locals too. Both runtimes gained instance-bound handlers:
  `rp_bind_event_indirect_this` for the interpreter, closures for native
  code.
- **Method pointers:** `CODEPTR(obj.Method)` and `BIND p TO TType.Method`,
  called as `CALLFUNC(p, obj, args…)`, on both backends.
- **RapidQ patterns inside TYPEs:**
  - a TYPE extending a component reaches that component's property objects
    (`Font.Size`, `Qlistviewex.font.size`, `obj.Canvas.Font.AddStyles`) and
    indexed sub-objects (`QlistviewEx.column(i).caption`);
  - `Application`, `Screen` and the other RapidQ global objects are never
    taken for component members;
  - fields of RapidQ object types RapidR doesn't implement yet (e.g.
    `QD3DVECTOR`) are property-bag objects, as before.
- **Debugger:** the IDE shows objects with their type, id and field names.
- 116 of the 386 RapidQ examples compile (was 113).

### Fixed
- **Web runtime events:** a handler with 2–5 parameters bound to an event
  without arguments was never called; handlers may now also bind or fire
  events themselves. Both runtimes use one `fire` path.
- **Native:** assigning a whole array to a module-level array variable
  compiled wrongly.
- **Interpreter:** the compiler's own helper calls inside a CREATE block
  were taken for methods of the created component.

## [2.26.0] — 2026-09-26

### Added
- **Arrays of components**, in both backends. `DIM lbl(1 TO 3) AS QLABEL`
  creates one real component per element, with ids `lbl(1)`, `lbl(2)`, …;
  more dimensions work too (`grid(0,1)`), and so do local arrays.
  - Elements take properties, reads, method calls and event handlers:
    `lbl(i).Caption = …`, `lbl(i).OnClick = Handler`, where Sender says
    which element fired.
  - The id array is built in `rapidr_value::objects::object_ids`, and both
    runtimes create the components (`rp_component_array`).
- **Indexed sub-objects of components**, in both backends.
  `SB.Panel(0).Width = 100` works, and inside `CREATE SB … END CREATE` so
  does the bare `Panel(0).Width = 100`. It becomes the method
  `panel.width=`, and the runtimes keep these as the component's
  properties.

  Status-bar panels and list-view columns don't render them yet. The
  CREATE-block rule is shared: `rapidr_ast::qualify_create_body`.
- **QFILESTREAM on the shared stream code**, the same as QMEMORYSTREAM on
  every platform:
  - `Open` with fmCreate/fmOpenRead/fmOpenWrite/fmOpenReadWrite; all the
    Read*/Write*/ReadNum/WriteNum methods, Seek, LineCount, Size and
    Position; `CopyFrom` between file and memory streams in both
    directions;
  - on the desktop, writes go straight to the file, so nothing is lost
    without `Close`; on the web, `Open` reads the page's own files;
  - writing to a file opened with fmOpenRead is refused with a warning.

  Before, desktop file streams only read and wrote whole lines.
- **`Stream.Read(var)`** (both backends): reads as many bytes as the
  variable holds (a string: its length), per the manual's "Generic Read".
  It's compiled as `var = Stream.__read(var)` (`rapidr_ast::stream_read_assignment`).
- **Tests:**
  - conformance cases `component_arrays`, `indexed_subobjects` and
    `file_streams`;
  - `tests/native_gui_events.mjs` now builds each GUI fixture both
    natively and with `--interp` and requires identical results; its new
    fixture is an array of buttons sharing one OnClick handler.

### Fixed
- **`File.EOF` is True (-1) at the end, not 1.** With RapidQ's bitwise
  `NOT`, `WHILE NOT File.EOF` never ended.
- **`rapidr build --interp` honours `CARGO_TARGET_DIR`** when it builds and
  embeds the runner.
- **Native CREATE blocks** no longer treat the compiler's own helpers as
  methods of the created component.
- 113 of the 386 RapidQ examples compile now (was 101).

## [2.25.0] — 2026-09-26

### Added
- **Native builds compile object-oriented TYPEs** (RapidQ manual ch. 10) as
  real Rust. `crates/rapidr-codegen-rust/src/objects.rs` lowers objects to
  plain routines with the interpreter's model, so both backends behave the
  same:
  - methods (SUB/FUNCTION) using `This`, `Me`, the type's own name, bare
    field names and a leading `.`;
  - CONSTRUCTORs (base TYPE first) and inheritance (`EXTENDS`);
  - `PROPERTY SET`, `RESULT`, and `obj.Func` without parentheses;
  - composition (fields of TYPE or component type are their own objects);
  - EVENT blocks bound to each instance (`EVENT OnClick`,
    `EVENT Panel.OnClick`);
  - `CREATE x AS TType … END CREATE`, array fields of objects, and objects
    passed to and declared in SUBs.

  This works on the desktop and the web target. Every conformance case now
  passes on both backends: 76 of 76, none marked as a known gap.
- **SUB parameters typed as components** (`Sender AS QBUTTON`) work in
  native builds: `Sender.Caption = …` used to fail to compile.
- **Desktop test hooks:** `RAPIDR_TEST_EVENTS=b1.onclick,…` fires events
  and `RAPIDR_TEST_DUMP=b1.caption,…` prints properties, together with
  `RAPIDR_CAPTURE`. New test `tests/native_gui_events.mjs` builds an OOP
  GUI program natively and checks each instance's clicks and Sender.

### Fixed
- **Compiled event handlers receive the Sender.** A handler with a
  parameter now gets the firing component, as in the interpreter; it used
  to get Null. This applies to both runtimes.
- **Interpreter: a bare method call inside `CREATE x AS TType`** (e.g.
  `Describe`) calls the TYPE's method. It went to the component and
  warned.
- **Tests:** new conformance case `oop_create_arrays`; `oop_types`,
  `oop_property_set` and `oop_composition` pass natively.

## [2.24.1] — 2026-09-26

### Changed
- **Native is native, interpreted is interpreted.** `rapidr build` no
  longer switches to the embedded interpreter for object-oriented
  programs. Until the Rust backend compiles OOP TYPEs, a native build of
  such a program stops with a clear error that points to the interpreter
  (`rapidr build-bc` / `run-bc`, or `--interp`). `RAPIDR_STRICT_CODEGEN` is
  gone.

## [2.24.0] — 2026-09-25

### Added
- **Function pointers in native builds:** `BIND p TO Proc`,
  `CODEPTR(Proc)` and `CALLFUNC(p, args…)`. Each SUB/FUNCTION gets an id,
  and `CALLFUNC` dispatches through a generated table (BYREF parameters
  included). An unset or wrong pointer stops with the same run-time error
  as the interpreter.
- **`rapidr build` handles object-oriented programs.** When a program uses
  TYPEs with methods, CONSTRUCTOR, EVENT, EXTENDS or PROPERTY SET, which the
  Rust backend doesn't compile yet, `rapidr build` still produces a native
  executable (or web bundle) and says so. It uses the embedded bytecode
  interpreter, exactly as `--interp` does, so the program behaves
  identically. `RAPIDR_STRICT_CODEGEN=1` turns this off; the conformance
  suite uses it so it keeps testing the Rust backend itself.

### Fixed
- **`rapidr build --interp` never embeds a stale interpreter.** It used
  whatever `rapidrintr-runner` was already in `target/`, even one older
  than the CLI, so new features were silently missing. It now always has
  cargo bring the runner up to date (a quick no-op when nothing changed).
- **Tests:** `function_pointers` now passes on both backends.

## [2.23.0] — 2026-09-25

### Added
- **Native builds (`rapidr build`) run line labels, `GOTO`, `GOSUB` and
  `RETURN`.** A routine that uses them is emitted as a state machine
  (`crates/rapidr-codegen-rust/src/jumps.rs`). This covers the main
  program, SUBs and FUNCTIONs:
  - backwards and forwards jumps, line numbers, nested GOSUB;
  - labels inside FOR/WHILE/DO/IF bodies;
  - GOSUB inside IF and loops, and `EXIT` out of such loops.

  Only statements that contain a jump target or a GOSUB are flattened;
  everything else stays ordinary Rust.
- **`STATIC` variables in native builds:** each becomes a program-wide slot
  for its routine, created once, so every call and recursion shares it.
- **Native builds report the same compile errors as the interpreter**, with
  line and column, e.g. an unknown SUB or a missing label. `rapidr build`
  runs the bytecode compiler's checks first. Errors about what only the
  interpreter lacks (DLL calls, raw memory, Windows APIs) don't stop a
  native build (`rapidr_bcgen::error_applies_to_native_builds`).

### Fixed
- **Native `SELECT CASE` inside a loop** no longer fails to compile ("value
  moved"): the tested value is copied.
- **Tests:** `gosub_goto`, `gosub_advanced`, `label_not_found`,
  `unknown_sub_error` and `static_vars` now pass on both backends (no
  longer marked xfail for codegen). New case `goto_structured`.

## [2.22.0] — 2026-09-25

### Added
- **In-page dialogs on the web, with every button.** When the bytecode
  interpreter runs a program (the IDE preview and exported bundles),
  `MESSAGEBOX`, `MESSAGEDLG`, `SHOWMESSAGE` and `INPUT` open a modal
  dialog in the page (`crates/rapidr-runtime-web/src/dialog_web.rs`):
  - it has a title and all the buttons, e.g. Yes · No · Cancel and
    Abort · Retry · Ignore;
  - the first button is the default, and Escape means "closed", as on the
    desktop.

  The program waits for the answer, in the main program or in an event
  handler. The VM suspends (`VmError::Suspended`, `Host::suspend_requested`)
  and `Vm::resume_with(answer)` continues it with its state intact, the way
  the debugger's pause already worked. Where waiting isn't possible (the
  Rust-compiled web build, or a handler fired from inside another VM call),
  the browser's dialogs are still used.
- **INPUT on the web:** an in-page text field showing the prompt; the
  typed line is echoed in the output. Console programs in exported bundles
  can now ask questions.

### Fixed
- **`INPUT` follows the RapidQ manual (ch. 6.4) on both backends:**
  - the prompt is printed (it was dropped);
  - a whole line is read;
  - the line is stored as text or a number according to the variable: its
    DIM type, else its suffix (`$`, `%`, `#`, …), else a number when the
    line is one. Before, `INPUT age` stored the string "41", so `age + 1`
    gave "411".

  The rule is shared in `rapidr_value::input_value`; the Rust backend
  lowers INPUT through `rapidr_ast::input_assignment`, which also fixes
  INPUT into global array elements there.
- **Tests:** the conformance runner feeds an optional `name.input` file to
  the program; new case `input_line`. `tests/web_ide_dialogs.mjs` now
  checks the in-page dialogs, including one opened from a button handler.
  `tests/web_bundle_console.mjs` checks INPUT in a bundle. A VM unit test
  covers suspending inside a function.

## [2.21.0] — 2026-09-25

### Added
- **Exported web bundles show `PRINT` output on the page.** This covers
  both `rapidr bundle-bc` and the IDE's Build. The console
  (`web-ide/bundle_console.js`) uses the same ANSI screen as the IDE's
  Output panel, so `CLS`, `COLOR` and `LOCATE` work:
  - it appears on the first PRINT;
  - it fills the page for a console program, and docks at the bottom when
    the program also shows forms;
  - it works under the IDE bundle's Content-Security-Policy.

  Before, bundles only wrote to the browser's developer console. The web
  runtime now also passes each printed text, exactly as printed, to
  `window.__rapidr_print` when a page defines it.
- **Tests:** `tests/web_bundle_console.mjs` (a console program and a form
  program built with the CLI). `tests/web_ide_e2e_build.mjs` now also
  checks the console in an IDE-built bundle.

## [2.20.0] — 2026-09-25

### Added
- **RapidQ's non-visual objects `QFONT`, `QMEMORYSTREAM`, `QBITMAP` and
  `QIMAGELIST`** (manual, Appendix A), no longer empty placeholders. They are
  written once, in `rapidr_value::objects`, and the desktop and web runtimes
  both use that code, so the two behave the same:
  - `QFONT`: Name, Size, Color, Bold/Italic/Underline/StrikeOut, `AddStyles`,
    `DelStyles`. `Label.Font = Font` applies the font to the component, on
    the desktop as well as on the web (desktop widgets didn't apply fonts
    before).
  - `QMEMORYSTREAM`: Position, Size, LineCount; `WriteStr`/`ReadStr`,
    `WriteLine`/`ReadLine`, `WriteNum`/`ReadNum` (the `Num_*` types),
    generic `Write`, `Seek`, `CopyFrom` (from another memory stream or a
    QFILESTREAM), `Close`.
  - `QBITMAP`: Width/Height, `Pixel(x, y)` (read and assign), `PSet`,
    `Line`, `Rectangle`, `FillRect`, `Circle` (outline or filled),
    `RoundRect`, `Paint` (flood fill), `Draw`, `CopyRect`, `StretchDraw`,
    Transparent/TransparentColor, `LoadFromFile`/`SaveToFile` and
    `LoadFromStream`/`SaveToStream` (uncompressed BMP, read and written by
    RapidR itself, with no new dependencies). `.BMP` gives the image as a
    `data:` URL, so `Image.BMP = Bitmap.BMP` works anywhere.
  - `QIMAGELIST`: Width/Height/Count/Masked, `AddBMPFile`/`AddBMPHandle`/
    `Insert…` (strips wider than the list are split into images, and the
    mask color becomes transparent), `GetBMP`, `Draw` onto a bitmap,
    `Delete`, `Clear`.
- **`Canvas.Draw(x, y, Bitmap)`** on the desktop and the web, keeping the
  bitmap's transparent color. On the web, `Bitmap.LoadFromFile` reads files
  shipped with the page, and `SaveToFile` keeps the file for the session.
- **Assigning to an indexed property**, e.g. `Bitmap.Pixel(x, y) = c`, calls
  the component's method with the value as its last argument (both
  backends).
- **Built-in RAPIDQ.INC:** `soFrom*`, `Num_*`, `fs*` font styles, `clNone`,
  `clDefault`, `pf*`.
- **Checking desktop rendering (for tests):** with `RAPIDR_CAPTURE=<prefix>`
  set, a desktop program saves each open window (forms and dialogs) as
  `<prefix>-N.bmp` and exits. This needs no screen-recording permission.
- **Tests:** conformance case `rapidq_objects`; IDE suite
  `tests/web_ide_objects.mjs`; unit tests for the BMP codec, drawing, streams
  and image lists.

### Fixed
- **Desktop canvas drawing is relative to the canvas**, as in RapidQ, and
  clipped to it. It used window coordinates, so on a canvas at Left = 100,
  `Line 0, 0, …` started at the window's corner.
- **Desktop `MESSAGEBOX`/`MESSAGEDLG`:** Enter now chooses the first button
  (Yes/OK), as in RapidQ. FLTK's stock dialog defaulted to the middle one,
  which was "No" on Yes/No/Cancel.
- **Desktop labels are left-aligned** by default (RapidQ's taLeftJustify),
  and honour `Alignment` 1 (right) and 2 (center). They were centered.
- **Web labels:** Underline and StrikeOut can be on together.

## [2.19.0] — 2026-09-25

### Added
- **`MESSAGEBOX(text, title, flags)` and `MESSAGEDLG(text, mtType, mbButtons,
  0)`** (RapidQ manual), with real buttons and the documented return values
  (IDOK/IDYES/…, mrOk/mrNo/…). The button logic lives in
  `rapidr_value::dialogs`:
  - desktop builds show an FLTK dialog with up to three buttons and a title;
  - the web uses the browser's blocking dialogs, labelled when OK/Cancel stand
    for other answers ("OK = Yes, Cancel = No"). Browsers offer at most two
    buttons, so a third (Yes/No/*Cancel*) isn't available there.
- **Built-in RAPIDQ.INC:** the `mt*` and `mb*` MessageDlg constants.
- **Tests:** IDE suite `tests/web_ide_dialogs.mjs`; bitwise operator checks
  in `operators_rapidq`.

### Fixed
- **`AND`, `OR`, `XOR` and `NOT` are bitwise, as in RapidQ.** The manual
  gives `5 OR 3 = 7` and `NOT -1 = 0`. They were logical, so
  `MB_YESNO OR MB_ICONQUESTION`, or any combined flags, gave -1. Comparisons
  are -1/0, so conditions behave as before. One change, in the shared value
  layer, fixes both backends.

## [2.18.0] — 2026-09-25

### Added
- **`REDIM a(n) AS T`** (RapidQ manual) on both backends:
  - the array is resized in place, so every variable holding it sees the new
    size;
  - each element whose index still fits is kept;
  - without an earlier DIM, it creates the array;
  - VB's `REDIM PRESERVE` is accepted;
  - one implementation, `rapidr_value::redim`, shared by both backends.
- **`a INV m`:** the modular inverse (`3 INV 26` = 9, 0 when there is none),
  at MOD's precedence.
- **Tests:** conformance case `redim_inv`.

## [2.17.0] — 2026-09-25

**Portable console, and honest Windows errors.** RapidQ's console statements
work on every platform and in the IDE. Calls into Windows itself say plainly
that RapidR doesn't emulate Windows, and what to use instead.

### Added
- **Console statements** (RapidQ appendix C) on both backends:
  - `CLS`, `COLOR [fg][, bg]` (QBasic colors 0-15), `LOCATE [row][, col]`,
    `CSRLIN` and `POS(0)`;
  - they emit standard ANSI/VT sequences (macOS, Linux and Windows 10+
    terminals), from one implementation in `rapidr_value::console`;
  - PRINT zones and the cursor ignore escape sequences.
- **The IDE Output panel renders them** (`web-ide/ansi_screen.js`): colors,
  cursor positioning and clearing, with printed text only ever inserted as
  text. Plain PRINT output looks and copies exactly as before.
- **Windows API errors name the portable alternative.** A `DECLARE … LIB`
  into a Windows system DLL (user32, gdi32, kernel32, winmm, odbc32, ws2_32,
  …) now reads: "'ShellExecute' is a Windows API function; RapidR runs on
  every platform and doesn't emulate Windows. Instead, run programs and open
  files with SHELL / SHELLWAIT". Hints cover ODBC → RSQLITE/RMYSQL, GDI →
  RCANVAS, window management → component properties, MCI → PLAYSOUND,
  Winsock → RSOCKET, WinINet → RHTTP and more. A program's own DLLs keep the
  native-build message.
- **Omitted arguments:** `INSTR(, a, b)`, `COLOR , 1`, `LOCATE , 5`.
- **Tests:** conformance case `console_ansi`, IDE suite
  `tests/web_ide_console.mjs`, and a unit test for the Windows API messages.

### Fixed
- **Bare `TIMER` in the interpreter.** It (and `TIME$`, `DATE$`, `COMMAND$`,
  `PI`) evaluated to Null when written without parentheses; native builds
  already called them. Both backends now share one list of such built-ins.

## [2.16.1] — 2026-09-25

**Open-source credits.** RapidR ships 478 open-source Rust libraries plus
native libraries such as FLTK and FreeType. All of them are now credited,
and the list can't drift out of date.

### Added
- **`THIRD_PARTY_NOTICES.md`:** every library in the shipped dependency
  graph, with version, license and upstream link. It's generated by
  `tools/third_party_notices.py` from `cargo metadata`, and CI fails when it's
  stale (`--check`). License clarifications come from `deny.toml`, so the
  notices and `cargo deny` agree.
- **`LICENSES.md` §7** credits the native C/C++ code that `-sys` crates build
  or link (FLTK, FreeType with its required notice, zstd, LZ4, zlib, OpenSSL,
  fontconfig, ALSA), which `cargo deny` can't see.
- **Web bundles** (`rapidr bundle-bc` and the IDE's Build) now include
  `LICENSE-RapidR.txt`, `THIRD_PARTY_NOTICES.md` and `LICENSES.md`, since they
  redistribute the runtime.
- **The IDE's About dialog** has an "Open-source credits" button, and the
  README has a credits paragraph.
- **ROADMAP policy:** no Windows-only compatibility; RapidQ features map to
  portable Rust; new dependencies must be permissive open source and be
  credited.

### Fixed
- **`web_ide_bugfixes`** expected an old About-dialog wording; it now checks
  the current credits text and the credits dialog.

## [2.16.0] — 2026-09-25

**RAPIDQ2.INC compiles.** RapidQ's official extension library (about 9,000
lines, 287 routines) now compiles and runs in the interpreter. It uses
composition, function pointers, SUBI/FUNCTIONI methods and more. Corpus: 96/386
programs compile, stricter than before: typos in a program's own code are
reported even in routines nothing calls.

### Added
- **Code the program never runs doesn't block it.** An error only counts in
  code the program can reach. Reachability follows every name mentioned,
  including handlers and BIND. This applies to:
  - native-only features (DLL calls, VARPTR, `@` to a DLL), anywhere;
  - unknown names and unsupported features, but only inside `$INCLUDE`d
    libraries. The program's own code is always fully checked.
- **Clearer messages for RapidQ built-ins RapidR lacks:** `LOCATE (a RapidQ
  built-in) isn't supported yet` instead of "Unknown SUB". The list comes from
  RapidQ's KEYWORD.LST and manual.
- **Composition** (manual 10.5), in the interpreter:
  - `Panel AS QPanel` fields become each instance's own component;
  - TYPE fields of TYPE type become objects;
  - `EVENT Panel.OnClick` handlers;
  - nested access at any depth (`A.Engine.Power = 5`, `b64.src.Close`,
    `A.Engine.Describe`);
  - arrays of objects (`image(1000) AS QBITMAP`);
  - a component's property objects (`.Canvas.Font.AddStyles`);
  - indexed sub-objects (`.column(i).caption`).
- **Function pointers:** `BIND ptr TO Proc` (or a TYPE method), `CODEPTR`/
  `CALLBACK`, and `CALLFUNC(ptr, …)`, with the new opcode `CallIndirect`
  (0x78). `BIND ptr TO Prototype` only gives the pointer a signature, as in
  RAPIDQ2.INC.
- **`SUBI`/`FUNCTIONI`** (manual ch. 9): variable arguments read through
  `ParamStr$(i)`, `ParamVal(i)`, `ParamStrCount` and `ParamValCount`. They
  work as TYPE methods and with `DECLARE`, on both backends.
- **`DATA`/`READ`/`RESTORE [label]`** on both backends: unquoted items are
  text unless numeric, and the table is shared by all DATA lines of the
  program.
- **`$ESCAPECHARS ON`:** `\n \t \" \\ \a \b \f \r \v`, `\65` and `\x41`.
- **`SWAP a, b`, and `SHL`/`SHR`** (32-bit, at `*` precedence).
- **Syntax:**
  - dotted names: `DECLARE SUB SLEEP.ms LIB …` and `FUNCTION Screen.MousePresent`
    (called as `Screen.MousePresent`);
  - names starting with digits (`SUB 01click`);
  - keywords as parameter names (`type AS LONG`);
  - `FOR i = 1 TO 3: PRINT i: NEXT` on one line;
  - `FOR THIS.x = …`;
  - `CASE = x`;
  - `< =` / `> =` / `< >` written with spaces;
  - `_ ' comment` continuations;
  - `WITH TypeName` around a TYPE's members;
  - `EXIT SUBI` / `EXIT FUNCTIONI` / `EXIT EVENT` / `EXIT PROPERTY`;
  - a SUB closed by `END FUNCTION` (and vice versa);
  - `SIZEOF(SINGLE)`;
  - array fields with bounds (`Colors(1 TO 16)`).
- **RapidQ objects RapidR has no component for yet** (QFONT, QBITMAP,
  QIMAGELIST, QMEMORYSTREAM, …) are accepted as types: generic objects whose
  missing methods warn at run time.
- **Tests:**
  - conformance cases `subi_functioni`, `data_read`, `oop_composition` and
    `function_pointers`;
  - `rapidq_syntax` and `operators_rapidq` extended;
  - reachability unit tests.

### Fixed
- **`BIND ptr TO Proc` did nothing.** The interpreter only printed a warning
  and native builds wrote a comment, so the statement was silently skipped.
  The interpreter now implements it; native builds report it clearly.
- **Labels only used by `RESTORE`** no longer stop native builds.

## [2.15.0] — 2026-09-25

**Real RapidQ programs.** The original RapidQ distribution (386 example
programs, 126 include files and the manual) is now the compatibility corpus.
`tools/rapidq_corpus.py` compiles every program and ranks what blocks them.
46/386 compiled at the start of this release and 82/386 at the end. Semantics
come from the RapidQ manual, not guesses.

### Added
- **Includes like RapidQ on Windows:**
  - `RAPIDR_INCLUDE_PATH` names RapidQ's `include\` directory.
  - `\` separators and case-insensitive names work.
  - Absolute paths (`c:\rapidq\include\windows.inc`) resolve by trying shorter
    endings of the path.
  - Windows-1252 source files are decoded.
  - `$DEFINE`s and `$MACRO`s from an include reach the program that includes it.
  - `WIN32` is predefined, as in RapidQ.
- **Errors in included code point at the include file and line.** Before, they
  showed a shifted line in the main file.
- **VB conditional compilation:** `#If … Then`, `#ElseIf`, `#Else`, `#End If`
  and `#Const`.
- **The full RapidQ `DIM` grammar:**
  - `DIM a AS INTEGER, s AS STRING`;
  - `DIM (a, b, c)(5) AS LONG`;
  - `DIM x AS INTEGER = 5`;
  - `AS STRING * 20`.
- **`DEFINT`, `DEFSTR`, `DEFLNG`, `DEFDBL` and the rest of the DEFxxx family,**
  with initializers: `DEFINT a(1 TO 3) = {1, 2, 3}` is filled in memory order,
  as the manual describes.
- **`STATIC` variables** in SUBs and FUNCTIONs, in the interpreter. The value
  survives between calls and is shared by recursive calls, exactly as in the
  manual's example.
- **Operators and syntax:**
  - `i++`, `i--`, `x += y`, `-=`, `*=`, `/=`, `&=`;
  - string index `s$[i]`;
  - string subtraction `"jello" - "l"` gives `"jeo"`;
  - `a NOT= b`;
  - literal type suffixes `0&`, `1.5!`, `2#`;
  - `CASE 1: stmt` and `CASE ELSE : stmt`;
  - `Foo : Bar` calls;
  - VB `Public`/`Private`/`Global` modifiers.
- **`@var` passes a variable by reference** (manual 3.5, `StrCat(@A$, "!")`),
  on both backends. For a DLL call, native builds pass its address.
- **Array parameters:** `SUB Fill (list() AS STRING)`.
- **SUBs named like file keywords** (`SUB Close`, `DECLARE SUB Open`), which
  RapidQ doesn't reserve.
- **`RESULT = value`** in FUNCTIONs.
- **Accented identifiers** such as `Précédent`.
- **String continuation:** a string can continue onto the next line with `_`.
- **RapidQ OOP from manual chapter 10 (interpreter):**
  - `Field AS LONG PROPERTY SET Setter` with `PROPERTY SET Setter (v) … END PROPERTY`;
  - the type's name standing for the instance (`TForm.Focus`, `WITH TForm`);
  - `EXTENDS QObject`;
  - `TYPE X AS QFORM`;
  - `PUBLIC:`/`PRIVATE:`/`PROTECTED:` sections;
  - `DECLARE` lines and `AS EVENT(…)` fields inside a TYPE;
  - `obj.Func` calls a FUNCTION method without parentheses.
- **`True`/`False` in the built-in RAPIDQ.INC**, 1 and 0 as in the real one.
- **AST walkers** `rapidr_ast::walk`, `walk_expressions_mut` and
  `ref_argument_positions`.
- **Tests:**
  - conformance cases `dim_forms`, `rapidq_syntax`, `static_vars`,
    `operators_rapidq`, `with_result` and `oop_property_set`;
  - preprocessor tests for includes and `#If`.

### Changed
- **Comparisons print as `-1`/`0`** (RapidQ has no boolean type), not
  `True`/`False`.
- **Precedence follows the manual.** `NOT` binds looser than comparisons, so
  `IF NOT x = 5` means `NOT (x = 5)`. `MOD` binds looser than `*` and `/`.
- **`DIM a, b AS LONG` makes `a` a VARIANT.** The manual says only `b` is
  LONG. Before, both were LONG.
- **Unterminated strings end at the end of the line,** as RapidQ does,
  instead of being an error.
- **Native builds reject `STATIC` inside a SUB/FUNCTION** with a clear
  message, like OOP TYPEs.

### Fixed
- **Typed variables start at their type's default in the interpreter.** An
  uninitialised `DIM n AS INTEGER` printed as empty; it now prints `0`.
- **`WITH obj … END WITH` works in the interpreter.** Every `.Member = v`
  inside it silently did nothing.
- **Bare FUNCTION names are called** (`y = Five + 1`) on both backends.
  Before, they read an empty variable.
- **Every compile error has a line number.** Errors raised while lowering a
  statement (e.g. unsupported nested member assignment) used to stop
  compilation without a position; they're now reported with their line and
  compilation continues.
- **The lexer no longer crashes** on non-ASCII text right after a line break.

## [2.14.0] — 2026-09-25

**RapidQ object-oriented TYPEs** in the bytecode interpreter (IDE preview and web
bundles). The ROADMAP's `rq3.bas` (`TYPE TMyForm EXTENDS QFORM` with an `EVENT OnClick`
block and a `CONSTRUCTOR`) now runs as written. Checked in the browser: the form opens
at 200 px with caption "Custom", and each click updates the caption with the
instance's own counter.

### Added
- **`TYPE … EXTENDS <component>`** creates a real component. The fields, methods,
  events and constructor all belong to the type, and every instance gets its own
  `This`.
- **`EVENT Name … END EVENT`** blocks inside a TYPE, also written `EVENT Name(params)`
  and `EVENT(Name)`. They are wired to the instance when it's created. A missing
  `END EVENT` is an error.
- **Methods** (`SUB`/`FUNCTION` inside a TYPE), called as `obj.Method args` or
  `x = obj.Func(...)`.
- **`CONSTRUCTOR … END CONSTRUCTOR`**. Constructors run when the instance is created,
  base type first, so derived types can override defaults.
- **Inheritance between user types** (`TYPE TLoud EXTENDS TCounter`). Methods,
  fields and constructors are inherited, and circular `EXTENDS` is guarded.
- **Implicit members:** inside TYPE code, a bare field or component property name
  (`Caption = "x"`, `Count = Count + 1`) means `This.<name>`. Locals, globals and
  functions still win.
- **`Sender` in event handlers:** handlers assigned with `OnClick = MySub` receive
  the firing component. `SUB MySub (Sender AS QBUTTON)` can then set
  `Sender.Caption`.
- **Objects as values:** a TYPE or component instance can be passed to a SUB
  (`SUB UseIt (obj AS TCounter)`) and used through the parameter: fields, methods,
  and component properties and methods.
- **Array fields** in TYPEs (`Names(2) AS STRING`), read and written as `obj.Names(i)`.
- New opcodes for this: `GetPropDyn` (0x75), `SetPropDyn` (0x76) and
  `CallMethodDyn` (0x77). They access members of an object whose identity is only
  known at run time (`This`, `Sender`, parameters).
- Tests:
  - conformance case `oop_types`;
  - parser tests for EVENT, methods and constructors;
  - `tests/web_bundle_oop.mjs`, a browser test covering per-instance event state
    and `Sender`.

### Changed
- Unknown lines inside a TYPE are now errors ("Unexpected 'X' inside TYPE T").
  Before, they were silently skipped. The same applies to `PROPERTY` declarations,
  which aren't supported yet.
- Native builds (`rapidr build`) reject TYPEs that use `EXTENDS`, methods, EVENTs or a
  CONSTRUCTOR with one clear `compile_error!`. It says to use the bytecode
  interpreter instead of producing confusing rustc errors.

### Fixed
- Assignments inside a `CREATE … END CREATE` body (`Caption = "x"`) no longer register
  a global variable with the property's name.
- The VM ignores surplus arguments to a SUB instead of writing past its locals.

## [2.13.0] — 2026-09-24

**RapidQ source compatibility:** classic RapidQ programs compile and run unchanged.
Checked with a RapidQ GUI program (`$INCLUDE "RAPIDQ.INC"`, `CREATE Form AS QFORM`, a
nested `QBUTTON`, `Center`, `OnClick`, `ShowMessage`) running in the browser: the form
is centered, the button's handler runs, and the caption updates.

### Added
- **RapidQ component names:** `QFORM`, `QBUTTON`, `QSTRINGLIST`, … (any `Qxxx` whose
  `Rxxx` exists, plus `QGAUGE` → `RPROGRESSBAR`) are accepted in `DIM`, `CREATE`,
  `TYPE … EXTENDS` and parameter types. Before, `QFORM` silently created nothing.
  The component list now lives in one place (`rapidr_ast::COMPONENT_TYPES`) instead of
  a copy per backend.
- **Built-in `RAPIDQ.INC`:** `$INCLUDE "RAPIDQ.INC"` works without the file. It
  provides the RapidQ/Delphi constants: colors (BGR, as RapidR's runtimes expect),
  `mr*`, `MB_*`/`ID*`, `bs*`, `ws*`, `al*`, `mb*`, `fm*`, `VK_*`. It expands to one line,
  so error line numbers after it stay correct. A real `RAPIDQ.INC` next to the program
  still takes precedence.
- **`?` as shorthand for `PRINT`**.

### Fixed
- Native builds: a forward declaration (`DECLARE SUB Foo (…)` for a SUB defined later)
  generated a duplicate function and failed to compile.

### Verification
- New conformance cases `question_mark_print` and `rapidq_program`; 38 of 42 backend
  runs pass (the 4 known failures are codegen GOTO/GOSUB and its unknown-SUB message).
- Parser tests for Q-name mapping; a preprocessor test for the built-in RAPIDQ.INC.
- RapidQ GUI program run in Chromium as a web bundle. 42/44 examples compile; all IDE
  suites pass.

### Known issues
- `$INCLUDE` of a real file splices its lines in, so errors reported after it point at
  the wrong line (source maps needed).

## [2.12.0] — 2026-09-24

Classic BASIC control flow: line labels, `GOTO`, `GOSUB`/`RETURN`, and `END`. The bytecode
interpreter (IDE, `build-bc`/`run-bc`, bundles, `--interp`) now passes every conformance
case.

### Added
- **Line labels, `GOTO` and `GOSUB` / `RETURN`** in the bytecode interpreter.
  - Labels can be `Name:` or line numbers (`100 PRINT …`), anywhere in the main program
    or inside a SUB/FUNCTION. Jumps stay within their routine.
  - GOSUB can nest; `RETURN` goes back to the latest GOSUB, and otherwise returns
    normally.
  - `Name:` stays a call when `Name` is a SUB or builtin (`DoEvents: x = 1`).
  - A missing or duplicate label is a compile error with its location.
  - New opcodes `Gosub` and `GosubRet`.
- Native builds (Rust codegen) refuse labels/GOTO/GOSUB with a clear compile error
  pointing to the interpreter, instead of generating code that runs wrongly.
  State-machine lowering for codegen is planned.

### Fixed
- **A bare `END` statement was silently ignored**, so programs ran on into the code
  after it (typically their GOSUB subroutines). It now ends the program. In the
  interpreter it also stops execution in the browser, where the END builtin only
  logged a message.

### Verification
- Conformance: 34 of 38 backend runs pass. The VM passes all 19 cases; the 4 known
  failures are codegen's GOTO/GOSUB refusal (3 cases) and its unknown-SUB message.
  New cases: `gosub_advanced` (nested GOSUB, GOSUB in a SUB, backwards GOTO, line
  numbers, `DoEvents:`, END) and `label_not_found`.
- GOSUB and END checked in the browser IDE. Parser tests for labels, GOTO/GOSUB and
  bare END. 42/44 examples compile; all IDE suites pass.

## [2.11.0] — 2026-09-24

Real arrays, correct PRINT output, and more RapidQ compatibility, now identical on both
backends. The conformance suite passes 31 of 34 backend runs; only GOTO/GOSUB and
codegen's unknown-SUB message remain.

### Added
- **Real BASIC arrays** (`Value::Array`), shared by the bytecode VM and the Rust codegen:
  - any number of dimensions and real bounds (`DIM a(10)` is 0 to 10;
    `DIM b(1 TO 5, 3)`), with each element set to the type's default;
  - `LBOUND(a [, dim])` / `UBOUND(a [, dim])`;
  - arrays passed to a SUB are shared, as in BASIC;
  - an out-of-range index is a run-time error ("Subscript out of range"), not a silent
    `null`.

  Before this, interpreter arrays were comma-separated strings: a string element
  containing a comma corrupted the array, numbers came back as strings, `DIM` didn't
  allocate, and 2-D arrays didn't work. Codegen arrays were 1-D only, and `LBOUND`/
  `UBOUND` always returned 0.
- **PRINT separators and print zones.** `;` joins items, `,` moves to the next 14-column
  zone (as in QBasic and VB), and only a trailing `;` or `,` keeps the cursor on the line.
  Before, any `;` suppressed the newline, codegen put spaces between items, and the VM
  put none.
- **`INC x [, n]` / `DEC x [, n]`** (RapidQ), on both backends; works for variables and
  array elements.
- **`REPLACESUBSTR$(s, find, replacement)`** (RapidQ), on both backends.
- Arrays appear in the debugger's variable view as lists.

### Fixed
- **String functions count characters, not bytes** (`LEN`, `MID$`, `LEFT$`, `RIGHT$`,
  `INSTR`, `RINSTR`, `INSERT$`, `DELETE$`), so text such as `"héllo"` or `"ñandú"` works.
  Before, it gave wrong results or crashed the program. `ASC` returns the character code
  (`ASC("é")` = 233, matching `CHR$`). These functions now live in one shared module
  (`rapidr_value::strings`) instead of a copy per runtime.
- **IDE Output panel:** every PRINT used to add a blank line, and `PRINT "a";` broke the
  line. Browser output is now line-buffered, and a partial line is flushed when the
  program or event handler finishes.

### Changed
- Bytecode: `NewArray`, `AGet` and `ASet` take a dimension/index count, and `ASet`
  updates in place; new `PrintZone` opcode. Bytecode from older compilers must be
  recompiled; the IDE and bundles always compile fresh.
- Codegen: arrays are ordinary variables holding `Value::Array` (`rp_get`/`rp_set`);
  the separate `GARRS` store is gone.

### Verification
- `cargo test --workspace`, including new tests for the shared string module, and
  conformance: 31 pass / 3 known failures / 0 failed.
- The array-using examples `test_array`, `hang_test` and `ide` (2,300 lines) build with
  codegen, and `test_array` prints the same values on both backends. 42/44 examples
  compile to bytecode (the other 2 are native-only).
- All IDE suites pass. IDE Output checked for `;`, `,` and trailing partial lines.

### Open questions (need real RapidQ to confirm)
- The 14-column print zone width, `INSERT$` argument order, and how non-whole numbers
  are formatted (`0.1 + 0.2`).

## [2.10.1] — 2026-09-24

Correctness fixes found by the conformance suite (now 20 of 32 backend runs pass,
up from 11).

### Fixed
- **`CASE 2, 3` (several values in one CASE) restarted the program in an infinite loop**
  in the bytecode interpreter, because of an unpatched jump to address 0. `SELECT CASE`
  is rewritten: any item matches, then the next CASE, then CASE ELSE.
- **`CASE 1 TO 5` and `CASE IS > 10`** were misparsed (`TO 5` became a stray statement;
  `IS` was treated as a variable), so both backends picked the wrong branch. They're now
  proper case items in the parser, VM and codegen. Codegen uses BASIC comparisons, so
  `2` matches `2.0`.
- **`FOR … STEP -n` never ran** in the bytecode interpreter. The loop now tests the
  step's sign.
- **FUNCTIONs returning by name** (`Fact = n * Fact(n - 1)`) returned nothing in the
  bytecode interpreter. The function's name is now its result variable, returned at
  `END FUNCTION`, `EXIT FUNCTION` and a bare `RETURN`.
- **`BYREF` parameters** were ignored by the interpreter, and the codegen output didn't
  compile. Both now write back to the caller's variable (copy-in/copy-out, as VB does);
  the VM uses a new `LoadArgOut` opcode.
- **`EXIT SUB` / `EXIT FUNCTION`** inside a loop only left the loop, and outside one
  they were ignored. **`EXIT FOR`** inside a nested `WHILE` left the `WHILE` instead of
  the `FOR` (interpreter and codegen). Each EXIT now leaves the right construct, and a
  misplaced EXIT is a compile error.
- Whole-number results print without a decimal point: `2 ^ 10` shows `1024`, not
  `1024.0`.
- Codegen: BYVAL parameters can be assigned to (they're `mut`), as BASIC allows.

### Added
- Conformance case `exit_statements`.

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

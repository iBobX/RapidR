# Changelog

All notable changes to RapidR are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project uses [Semantic Versioning](https://semver.org/). Planned work lives in
[ROADMAP.md](ROADMAP.md); security finding IDs (`SEC-xx`) refer to it.

## [Unreleased]

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

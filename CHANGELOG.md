# Changelog

All notable changes to RapidR are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project uses [Semantic Versioning](https://semver.org/). Planned work lives in
[ROADMAP.md](ROADMAP.md); security finding IDs (`SEC-xx`) refer to it.

## [Unreleased]

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

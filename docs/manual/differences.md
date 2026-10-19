# Differences from RapidQ, and extensions

RapidR is an original implementation, written from the ground up in pure
Rust, and it is **compatible** with RapidQ: it is not RapidQ and not a copy of
it. The rule it follows: what programs written for RapidQ do, they keep
doing; RapidR's extensions are additive and never change what an existing
program does.

## Names

RapidR's components have their own names — `RForm`, `RButton`,
`RStringGrid` … — which RapidR Studio, the examples and this manual use.
**RapidQ's names (`QFORM`, `QBUTTON` …) are accepted everywhere, silently,
always**: an old program runs unchanged, and both names can be mixed in one
program. Each RapidQ name is its R name (`QBUTTON` = `RButton`), except
`QGAUGE` = `RProgressBar`, `QOUTLINE` = `RTreeView` and `COMPORT` =
`RComPort` ([the full list](reference/components.md)). RapidR's own
components (`RPlot`, `RSQLite`, `RJson` …) have only an R name. RapidQ's
other source extensions, `.rqw`, `.rqb` and `.rq`, are read like `.bas`.
To move a program to RapidR's names, [import it](importing-rapidq.md): a
converted copy, proved to compile exactly as the original, with a report.

## How compatibility is kept

- **Reference behaviour.** When the manual leaves a question open, a probe
  program settles it and RapidR follows the recorded result. That settled
  PRINT's number format, its comma, integer stores, rounding, the bit
  operators, division by zero, string functions' edge cases, component
  colours and more ([docs/rapidq-ground-truth.md](../rapidq-ground-truth.md)).
- **The conformance suite** (over 250 programs, each with its expected
  output) runs on all three runtimes: native, interpreted and in the browser.
- **The example corpus** (428 older example programs): RapidR
  compiles 358 of them; most of the rest call Windows DLLs directly or use
  OLE (below). On Windows every one is run, interpreted and as a native
  build, and their windows compared with the suite's reference results, and
  where each one stands is listed program by program in
  [rapidq-corpus-status.md](../rapidq-corpus-status.md). The programs that
  use only the API the older Windows and Linux versions shared run on macOS,
  Linux and the web too: 114 of them ran with no problem on each of macOS
  (interpreted and native) and Linux, and the web matches the desktop
  ([rapidq-corpus-status-unix.md](../rapidq-corpus-status-unix.md)).
- **The GUI** is checked by driving every test program's windows and
  comparing their pixels and accessibility trees: native against
  interpreted, the desktop against the browser, at 1× and 2× scale.

## What RapidR doesn't do

- **Windows API calls off Windows.** A program that calls Windows' DLLs
  (`DECLARE … LIB "user32"`, kernel32, gdi32, …, and RapidQ's
  `SENDMESSAGE`, `POSTMESSAGE`, `KILLMESSAGE`) makes those calls **when it
  runs on Windows**, natively and interpreted (see the language guide). On
  macOS, Linux and the web it compiles and runs until the first such call,
  which stops it with an error naming the function and, when there is one,
  RapidR's portable equivalent (`GetTickCount` → `TIMER`). RapidR doesn't
  emulate Windows. A few Windows calls RapidQ programs commonly make keep
  working on every platform because they have a clear meaning:
  `Shell_NotifyIcon` (the tray), the registry (RRegistry), `joyGetPosEx`
  names RDXJoystick.
- **What a 64-bit Windows program can't do.** RapidR programs are 64-bit:
  a 32-bit DLL shipped with an old program can't be loaded (the error says
  so; a 64-bit build of the DLL is needed), x86 machine code a program
  writes and runs through `CallWindowProc` can't run, and a TYPE whose
  fields hold pointers or handles as `LONG` doesn't match the 64-bit layout
  Windows expects for that structure. SUBs handed to a DLL as callbacks
  (`CODEPTR` for a window procedure, an enumeration) aren't supported yet.
  A control's `Handle` is RapidR's own number (RapidR draws its controls),
  so Windows messages sent to it go nowhere; a form's `Handle` is its real
  window. On a high-DPI screen the coordinates Windows' functions take and
  give are the screen's pixels, where RapidQ (not DPI-aware) got scaled
  ones.
- **OLE / COM** (`QOLECONTAINER`, `QOLEOBJECT`): programs that declare
  them compile; their methods do nothing and print a warning.
- RapidQ's undocumented `QD3DANIMATION` / `QD3DANIMATIONSET` (in its keyword
  list only): programs that declare them compile and
  `QDXSCREEN.CreateAnimation` / `CreateAnimationSet` take them, but as in
  RapidQ there is nothing to do with them (RapidQ gives them no keys and
  nothing that plays them).
- DOS-era port I/O.
- `ON ERROR` is accepted and ignored (it's VB's, not RapidQ's): a run-time
  error still ends the program.

## Deliberate differences

Where RapidQ's behaviour was a crash or depended on Windows, RapidR does
something defined instead, and says so:

- Hostile values never crash the runtime: a run-time error with its line
  instead ("Division by zero" where RapidQ raised an exception).
- Memory functions (`VARPTR`, `MEMCPY`, `MEMSET`, `MEMCMP`) are
  memory-safe: addresses are views of the program's own data; a bad one is
  a run-time error.
- An RDXJoystick with no joystick reads "not connected" rather than
  raising RapidQ's list-index error.
- `TextRect(Rect, x, y, S$, fc, bc)` draws: the text clipped to `Rect`,
  which a background `bc` fills first (as the Windows call under it does).
  RapidQ's own TextRect stops the program on every object tried (an access
  violation on a QBITMAP, a list-index error on a QCANVAS).
- Windows on every system are drawn by RapidR itself, in RapidR's own look
  by default (RapidQ's exact Windows look with `$THEME Classic`; sizes and
  places are RapidQ's in both), with real system menus, dialogs and
  clipboard where users expect them.
- Documents printed with `Printer` / `LPRINT` are PDFs sent to the system's
  printer (see [Components](components.md#the-global-objects)).
- **The font dialog names a system colour.** A font that never set a colour
  (a new `RFontDialog`, most components' fonts) has `clWindowText`. RapidQ's
  dialog lists it as "Custom"; RapidR's shows "Black" (`clWindow` "White"),
  and pressing OK without picking another colour keeps `clWindowText`, so the
  text keeps following the theme:

  ```basic
  FontDlg.GetFont(Label1.Font)     ' the list shows Black
  IF FontDlg.Execute THEN FontDlg.SetFont(Label1.Font)
  ```

## Extensions

Everything here is RapidR's own; a program that uses it no longer builds
with RapidQ's compiler.

**Language**

- VB's conditional compilation `#If` / `#ElseIf` / `#Else` / `#End If`,
  `#Const`.
- `i++`, `i--`, `+=` and friends, `CBOOL`, VB's `Public` / `Private` /
  `Global` (accepted).
- `$THEME`; `$APPTYPE WEB`; `#!/usr/bin/env rapidr` scripts.
- `RUSTSTART … RUSTEND` blocks of Rust in native builds.

**Components and members**

- RapidR-only components: `RSQLITE`, `RJSON`, `RHTTP`, `RSERVERSOCKET`,
  `RNUM`, `RDATAFRAME`, `RPLOT` and the linked data components `RCSV`,
  `RDATASOURCE`, `RDBGRID` ([Data science](data-science.md)), `RAI`
  ([The AI assistant and RAI](ai-assistant.md)), `RMARKDOWNVIEW`,
  `RCODEEDITOR`, `RDESIGNSURFACE`, `RMEMO`, `RUPDOWN`, `RDATETIMEPICKER`,
  `RTOOLBAR`, the web components ([reference](reference/components.md)).
- On existing components: `DataSource` and `DataField` on `REdit` and
  `RLabel`; `Paragraph` and `TextRTF` on `RRichEdit`
  ([Rich text](rich-text.md)); `Form.OnDropFiles`.
- On every component: `Anchors`, size constraints, `AccessibleName` /
  `AccessibleDescription`, `Scale` / `OnScaleChanged`; `Application.Theme`
  ([Components](components.md#rapidrs-additions-to-every-component)).
- SQL parameter binding (`Query sql, values…`, `AddParam`): no SQL injection.
- SVG images wherever RapidQ takes a bitmap.
- RDXJoystick's axes, buttons and events; QMIDI without a system
  synthesizer.

**Runtimes and tooling**

- Three runtimes from one source: native executables, the interpreter and
  the browser.
- RapidR Studio: a form designer, a code editor with IntelliSense, a
  debugger, linked data live in the designer, an AI assistant (off until
  you turn it on), on the desktop and in the browser.
- High-DPI drawing, screen-reader accessibility and themes for every
  program, old ones included.
- Programs are drawn in RapidR's own look by default (RapidR Studio's:
  light, dark or high contrast as the system is set, the default font in
  Inter); `$THEME Classic` draws RapidQ's exact Windows look. Sizes,
  places and the colours a program reads back are RapidQ's in every look
  ([Themes](components.md#themes)).
- The RapidR Runtime with file types; standalone executables that need
  no Rust.
- Every build carries its third-party notices.

## Planned

The roadmap ([ROADMAP.md](../../ROADMAP.md)) lists what comes next, among it:
the manual, Studio and the examples in Spanish, Portuguese and Chinese as well
as English; `$THEME native` (the look of each system); RapidR running on an
ESP32; local AI models and a drop-in chat component; more data sources (Excel,
Parquet, PostgreSQL …); `$UNSAFEMEM` and calling native libraries on macOS and
Linux as RapidQ calls DLLs; mobile. None of these is in 2.119.0.

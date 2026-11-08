# Compatibility notes

RapidR is an original implementation, written from the ground up in pure
Rust.

RapidR includes a compatibility layer to import older programs written for RapidQ ([Importing older programs](importing-rapidq.md)).

The rule the layer follows: what an older program does, it keeps doing;
RapidR's extensions are additive and never change what an existing program
does.

## Names

RapidR's components have their own names — `RForm`, `RButton`,
`RStringGrid` … — which RapidR Studio, the examples and this manual use.
**The classic names (`QFORM`, `QBUTTON` …) are accepted everywhere,
silently, always**: an older program runs unchanged, and both names can be
mixed in one program. Each classic name is its R name (`QBUTTON` =
`RButton`), except `QGAUGE` = `RProgressBar`, `QOUTLINE` = `RTreeView` and
`COMPORT` = `RComPort` ([the full list](reference/classic-names.md)).
RapidR's own components (`RPlot`, `RSQLite`, `RJson` …) have only an R name.
The older source extensions, `.rqw`, `.rqb` and `.rq`, are read like `.bas`.
To move a program to RapidR's names, [import it](importing-rapidq.md): a
converted copy, proved to compile exactly as the original, with a report.

## How compatibility is kept

- **Reference behaviour.** When a question about an older program's
  behaviour is open, a probe program settles it and RapidR follows the
  recorded result. That settled PRINT's number format, its comma, integer
  stores, rounding, the bit operators, division by zero, string functions'
  edge cases, component colours and more
  ([the ground-truth record](../rapidq-ground-truth.md)).
- **The conformance suite** (over 250 programs, each with its expected
  output) runs on all three runtimes: native, interpreted and in the browser.
- **The example corpus** (428 older example programs): RapidR
  compiles 358 of them; most of the rest call Windows DLLs directly or use
  OLE (below). On Windows every one is run, interpreted and as a native
  build, and their windows compared with the suite's reference results, and
  where each one stands is listed program by program in
  [the corpus status on Windows](../rapidq-corpus-status.md). The programs
  that use only the API the older Windows and Linux versions shared run on
  macOS, Linux and the web too: 114 of them ran with no problem on each of
  macOS (interpreted and native) and Linux, and the web matches the desktop
  ([the corpus status on macOS, Linux and the web](../rapidq-corpus-status-unix.md)).
- **The GUI** is checked by driving every test program's windows and
  comparing their pixels and accessibility trees: native against
  interpreted, the desktop against the browser, at 1× and 2× scale.

## Older programs' habits RapidR keeps

### A CONST written again: `$OPTION WEAKTYPE`

A classic program (`.bas`, `.rqw`, `.rqb`) can write a CONST again when
`$OPTION WEAKTYPE ON` is in force where the second CONST stands. The VB
constants files older programs `$INCLUDE` (a "Win32 API" file, say)
start with `$OPTION WEAKTYPE ON`, end with `$OPTION WEAKTYPE OFF` and write
some of the names `RAPIDQ.INC` has again; RapidR compiles them as the
classic compiler does, in the same order.

```basic
$OPTION WEAKTYPE ON
CONST Version = 1
CONST Version = 2             ' no error: from here Version is 2
CONST Version = "two"         ' the last one wins, of any type
$OPTION WEAKTYPE OFF
CONST Version = 3             ' error: CONST variable Version already defined
```

- **On** (where the second CONST is written): no error. The last value
  wins, of the same type or not, and a line between two CONSTs reads the
  value of the one before it. `Public Const` and `Private Const` are CONSTs;
  capitals don't matter (`a` is `A`). A CONST may also take the name of a
  variable or array you DIMmed, or of a SUB or FUNCTION written before it.
- **Off** (the default): a second CONST of a name is an error, whatever
  value it has, in the same file or another, `Public Const` too:
  `CONST variable ANSI_CHARSET already defined`. The option counts where the
  second CONST is written, so an include file that ends with it ON leaves it
  on. The order of the files matters: `$INCLUDE "rapidq.inc"` then the port
  compiles; the port first and then `rapidq.inc` stops at the first name
  `rapidq.inc` repeats. (`$OPTION VBDLL ON`, which those files also
  switch, does not do this.)
- **Only a CONST.** A name DIMmed twice (`Identifier B already used, try
  another name`), a DIM after a CONST, and a second SUB, FUNCTION or CREATE
  of a name stay errors. Anything but `ON` or `OFF` after the option is an
  error (`Option WEAKTYPE expected ON|OFF but got MAYBE`).
- **A `.rr` program has no such option**: a repeated CONST stays an error
  there, and says where the first one is. Write each name once: delete the
  repeated line, or leave out of the program the file that repeats it.

Two corners are not the classic compiler's. A SUB or FUNCTION written
*before* the second CONST and called after it reads the new value here (the
classic compiler fixes a constant where a routine is written, so the routine
keeps the older one); and `CONST A = A + 1`, a CONST defined from its own
earlier value, adds 1 to it here, where the classic compiler reads the name
as 0 there. A SUB or FUNCTION written *after* a CONST of its name is still an
error here (the classic compiler lets it through under the option, and the
routine then hides the CONST).

Try it: [examples/rapidq/weaktype](../../examples/rapidq/weaktype/weaktype.bas)
(a constants file and a program that write RAPIDQ.INC's names again).

## What RapidR doesn't do

- **Windows API calls off Windows.** A program that calls Windows' DLLs
  (`DECLARE … LIB "user32"`, kernel32, gdi32, …, and the classic
  `SENDMESSAGE`, `POSTMESSAGE`, `KILLMESSAGE`) makes those calls **when it
  runs on Windows**, natively and interpreted (see the language guide). On
  macOS, Linux and the web it compiles and runs until the first such call,
  which stops it with an error naming the function and, when there is one,
  RapidR's portable equivalent (`GetTickCount` → `TIMER`). RapidR doesn't
  emulate Windows. A few Windows calls older programs commonly make keep
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
  give are the screen's pixels, where an older program (not DPI-aware) got
  scaled ones.
- **OLE / COM** (`QOLECONTAINER`, `QOLEOBJECT`): programs that declare
  them compile; their methods do nothing and print a warning.
- The undocumented `QD3DANIMATION` / `QD3DANIMATIONSET` (named in the classic
  keyword list only): programs that declare them compile and
  `QDXSCREEN.CreateAnimation` / `CreateAnimationSet` take them, but there is
  nothing to do with them (no keys, and nothing that plays them).
- DOS-era port I/O.
- `ON ERROR` is accepted and ignored: a run-time error still ends the
  program.

## Deliberate choices

Where an older program's behaviour was a crash or depended on Windows, RapidR
does something defined instead, and says so:

- Hostile values never crash the runtime: a run-time error with its line
  instead (for instance "Division by zero").
- Memory functions (`VARPTR`, `MEMCPY`, `MEMSET`, `MEMCMP`) are
  memory-safe: addresses are views of the program's own data; a bad one is
  a run-time error.
- An RDXJoystick with no joystick reads "not connected" rather than
  raising a list-index error.
- `TextRect(Rect, x, y, S$, fc, bc)` draws on every object: the text clipped
  to `Rect`, which a background `bc` fills first (as the Windows call under
  it does).
- Windows on every system are drawn by RapidR itself, in RapidR's own look
  by default (the classic Windows look with `$THEME Classic`; sizes and
  places are the classic ones in both), with real system menus, dialogs and
  clipboard where users expect them.
- Documents printed with `Printer` / `LPRINT` are PDFs sent to the system's
  printer (see [Components](components.md#the-global-objects)).
- **A string with a character above 255 is text, not bytes.** A string is
  bytes whenever every character is up to 255: `WriteStr(CHR$(200), 1)`
  writes the byte 200 and a binary file copied with `ReadStr` / `WriteStr`
  is exact. RapidR also has characters above 255 (`"中文"`, `"😀"`), and a
  stream writes such a string whole as UTF-8, where a count of bytes would
  have cut it in the middle of a character. `WriteStr(S, n)` counts characters, so `WriteStr(S, LEN(S))`
  writes all of `S`. Text boxes and the code editor never save a `?` in
  place of a character their file's encoding lacks (they save UTF-8
  instead), and `WriteText` / `ReadAllText` are there for text on purpose
  (see [Text and bytes](language.md#text-and-bytes-accents-chinese-emoji)).
- **The font dialog names a system colour.** A font that never set a colour
  (a new `RFontDialog`, most components' fonts) has `clWindowText`. The
  dialog shows it as "Black" (`clWindow` "White"), and pressing OK without
  picking another colour keeps `clWindowText`, so the text keeps following
  the theme:

  ```basic
  FontDlg.GetFont(Label1.Font)     ' the list shows Black
  IF FontDlg.Execute THEN FontDlg.SetFont(Label1.Font)
  ```

### QFormEx

A common include, `Rapidq2.inc`, defines a form `QFormEx` that gets its
extra features (dropped files, a tray icon, transparency, the window's
messages) by writing machine code into memory and making it the window's
procedure. RapidR never runs code a program writes into memory, so it
replaces that form with its own [RFormEx](forms-ex.md), natively and on
every system: the program runs unchanged, and everything else in the
include stays as it is.

- RapidR recognises the include's `TYPE QFormEx EXTENDS QFORM` by its public
  members, whatever the file is called. A `QFormEx` with other members (an
  older or newer include, or your own class of that name) isn't guessed at:
  the compiler stops at that `TYPE` and says which members differ — rename
  your class (`MyFormEx`), or leave the `TYPE` out to use RapidR's.
- A form written `QFormEx` keeps the include's behaviour where it differs
  from `RFormEx`: an empty `DragZone` (as it starts) takes no file — set a
  zone; `Transparent` reads 0 until set, and the window stays as it is until
  then; `OnMouseWheel` gets 1 or -1 each time, the mouse's place on the
  screen and Windows' key flags (4 Shift, 8 Ctrl); on Windows `OnMinimise`
  comes just before the window minimises (elsewhere the system tells RapidR
  once it happened). `InheritOnShow` and `InheritOnClose` are accepted and
  do nothing: RapidR does that work itself.
- `OnMessage` hears what the include's did: every message but the ones that
  are the form's own events (see [the window's messages](forms-ex.md#the-windows-messages-onmessage)).
- Off Windows, `Rapidq2.inc` itself calls Windows (`GetVersion`, when it is
  loaded), so a program that includes it stops there on macOS, Linux and the
  web, as any program that calls Windows does.

## Extensions

Everything here is RapidR's own: only RapidR builds a program that uses it.

**Language**

- VB's conditional compilation `#If` / `#ElseIf` / `#Else` / `#End If`,
  `#Const`.
- `i++`, `i--`, `+=` and friends, `CBOOL`, VB's `Public` / `Private` /
  `Global` (accepted).
- `$THEME`; `$APPTYPE WEB`; `#!/usr/bin/env rapidr` scripts.
- `RUSTSTART … RUSTEND` blocks of Rust in native builds.

**Components and members**

- RapidR-only components: `RSQLITE`, `RJSON`, `RHTTP`, `RSERVERSOCKET`, `RHTTPSERVER` ([Networking](network.md)),
  `RNUM`, `RDATAFRAME`, `RPLOT` and the linked data components `RCSV`,
  `RDATASOURCE`, `RDBGRID` ([Data science](data-science.md)), `RAI`
  ([The AI assistant and RAI](ai-assistant.md)), `RMARKDOWNVIEW`,
  `RCODEEDITOR`, `RDESIGNSURFACE`, `RMEMO`, `RUPDOWN`, `RDATETIMEPICKER`,
  `RTOOLBAR`, `RSECRETS` ([Keeping secrets](secrets.md)), the web
  components ([reference](reference/components.md)).
- On existing components: `DataSource` and `DataField` on `REdit` and
  `RLabel`; `Paragraph` and `TextRTF` on `RRichEdit`
  ([Rich text](rich-text.md)); `Form.OnDropFiles`.
- On every component: `Anchors`, size constraints, `AccessibleName` /
  `AccessibleDescription`, `Scale` / `OnScaleChanged`; `Application.Theme`
  ([Components](components.md#rapidrs-additions-to-every-component)).
- SQL parameter binding (`Query sql, values…`, `AddParam`): no SQL injection.
- SVG images wherever a component takes a bitmap; drawn with `currentColor` or
  RapidR's colour tokens, they take the theme's colours, and
  `RImageList.AddSVGFile` adds them at the list's size
  ([Your own SVG icons](svg-icons.md)). A `$RESOURCE`'s name goes wherever
  a picture file goes (`RImage.Load RES`).
- Encrypted MySQL connections: RMySQL's `SSLMode`, `SSLCA` and `Encrypted`
  ([Databases](databases.md#encrypted-connections)).
- RDXJoystick's axes, buttons and events; QMIDI without a system
  synthesizer.

**Runtimes and tooling**

- Three runtimes from one source: native executables, the interpreter and
  the browser.
- RapidR Studio: a form designer, a code editor with completion and hover help, a
  debugger, linked data live in the designer, an AI assistant (off until
  you turn it on), on the desktop and in the browser.
- High-DPI drawing, screen-reader accessibility and themes for every
  program, old ones included.
- Programs are drawn in RapidR's own look by default (RapidR Studio's:
  light, dark or high contrast as the system is set, the default font in
  Inter); `$THEME Classic` draws the classic Windows look. Sizes,
  places and the colours a program reads back are the classic ones in every
  look ([Themes](components.md#themes)).
- The RapidR Runtime with file types; standalone executables that need
  no Rust.
- Every build carries its third-party notices.

## Planned

The roadmap ([ROADMAP.md](../../ROADMAP.md)) lists what comes next, among it:
the manual, Studio and the examples in Spanish, Portuguese and Chinese as well
as English; `$THEME native` (the look of each system); RapidR running on an
ESP32; local AI models and a drop-in chat component; more data sources (Excel,
Parquet, PostgreSQL …); `$UNSAFEMEM` and calling native libraries on macOS and
Linux as programs call DLLs on Windows; mobile. None of these is in 2.119.0.

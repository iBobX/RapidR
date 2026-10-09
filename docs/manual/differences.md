# Differences from RapidQ, and extensions

RapidR is not RapidQ, and not a copy of it: it is an original
implementation, written from the ground up in pure Rust, that is
**compatible** with RapidQ. The rule it follows: everything RapidQ has
behaves exactly as in RapidQ; RapidR's extensions are additive and never
change what an existing program does.

## How compatibility is checked

- **RapidQ's own compiler is the ground truth.** RC.EXE (Rapid-Q 2006)
  still runs in a Windows virtual machine; when the manual leaves a
  question open, a probe program is compiled with it and RapidR is made to
  print exactly what RapidQ prints. That settled PRINT's number format, its
  comma, integer stores, rounding, the bit operators, division by zero,
  string functions' edge cases, component colours and more
  ([docs/rapidq-ground-truth.md](../rapidq-ground-truth.md)).
- **The conformance suite** (over 150 programs, many with RapidQ's own
  output as the expected result) runs on all three runtimes: native,
  interpreted and in the browser.
- **RapidQ's example corpus**: of RapidQ's 386 example programs, the 123
  that use only the API RapidQ's Windows and Linux versions shared all
  compile, and run alike natively and interpreted. Most of the rest call
  Windows DLLs directly (below).
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
  `Shell_NotifyIcon` (the tray), the registry (QREGISTRY), `joyGetPosEx`
  names QDXJOYSTICK.
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
- A QDXJOYSTICK with no joystick reads "not connected" rather than
  raising RapidQ's list-index error.
- `TextRect(Rect, x, y, S$, fc, bc)` draws: the text clipped to `Rect`,
  which a background `bc` fills first (as the Windows call under it does).
  RapidQ's own TextRect stops the program on every object tried (an access
  violation on a QBITMAP, a list-index error on a QCANVAS).
- Windows on every system are drawn by RapidR in Windows' classic look,
  with real system menus, dialogs and clipboard where users expect them.
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
- **Console box characters.** RapidQ's console shows characters 128 to 255
  in the old DOS code page (`CHR$(201)` is ╔); RapidR's console shows the
  program's text as it is (`CHR$(201)` is É), so box-drawing examples
  written for DOS (`3DBOX`, `BATTLE`) show letters where RapidQ drew lines.

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
  `RNUM`, `RDATAFRAME`, `RPLOT`, `RCODEEDITOR`, `RDESIGNSURFACE`, `RMEMO`,
  `RUPDOWN`, `RDATETIMEPICKER`, `RTOOLBAR`, the web components
  ([reference](reference/components.md)).
- On every component: `Anchors`, size constraints, `AccessibleName` /
  `AccessibleDescription`, `Scale` / `OnScaleChanged`; `Application.Theme`
  ([Components](components.md#rapidrs-additions-to-every-component)).
- SQL parameter binding (`Query sql, values…`, `AddParam`): no SQL injection.
- SVG images wherever RapidQ takes a bitmap.
- QDXJOYSTICK's axes, buttons and events; QMIDI without a system
  synthesizer.

**Runtimes and tooling**

- Three runtimes from one source: native executables, the interpreter and
  the browser.
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

The roadmap ([ROADMAP.md](../../ROADMAP.md)) lists what comes next: the
full IDE (RapidR Studio) on the desktop and the web, with IntelliSense, a
debugger and AI assistance through an MCP server; an `RAI` component for
AI in your own programs; mobile. None of these is in 2.117.0.

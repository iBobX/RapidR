# Windows DLL calls, one address space, PEEK / POKE

Decided 2026-10-08: a RapidQ program that declares routines of Windows'
own DLLs (`DECLARE FUNCTION GetDC LIB "user32" …`) **really calls them when
it runs on Windows** — in native builds and in the interpreter alike — and
on macOS, Linux and the web it stops at the call with a clear error that
names the function and says it needs Windows. Before, the compiler refused
such programs everywhere.

Ground truth: RC.EXE in the Windows 11 VM (`tools/rc_probe.sh`; the probes
are in `tests/conformance/.work/csys/` while they ran: `decl`, `decl2`,
`peek`, `speek`, `hex`). What RapidQ does is written down here; RapidR does
the same, and its own additions are marked.

## 1. The call: RapidQ's DECLARE rules

Checked with RC.EXE (`decl/d1`, `decl2/d2`, `decl2/d5`, `decl/d3`):

| Parameter | RapidQ passes | After the call |
|---|---|---|
| `x AS LONG` (nothing said), `BYVAL x AS LONG` | the 32-bit value (**by value is the default**, unlike Visual Basic: `nSize As Long` given 64 made GetComputerName read address 64 and crash) | — |
| `BYREF x AS LONG` | the address of the variable | the variable holds what the DLL wrote (`nSize` came back 15) |
| `x AS STRING`, `BYVAL x AS STRING` | the address of the string's characters (a NUL after them) | the string holds what the DLL wrote, its length unchanged (`GetWindowsDirectoryA(s, 260)` on `s = SPACE$(260)`) |
| `x AS STRING * n` | the n characters' address (through `VARPTR(buf)` or straight) | the same |
| `lp AS TRect` (a TYPE) | the address of its packed fields — a UDT is always by reference | the fields hold what the DLL wrote (`GetCursorPos(p)`) |
| `BYVAL d AS DOUBLE` | 8 bytes by value (`floor(2.7)` from msvcrt: 2) | — |
| `BYVAL b AS BYTE` | broken in RapidQ (the API read 0) | — |
| the result | `AS LONG` / INTEGER / DWORD: the 32-bit value; `AS DOUBLE`: the double; `AS STRING`: the characters at the returned address | |

A DLL function the DLL doesn't export, or a DLL that isn't there, stops a
RapidQ program before it starts (a message box: `decl/d4`). RapidR resolves
each function the first time it is called and reports it as a run-time
error naming the DLL and the function.

### What RapidR does with that on 64-bit Windows

RapidR programs are 64-bit (x64 and ARM64; the VM is Windows 11 ARM), so
the DLLs are the 64-bit system DLLs: a pointer is 64 bits, a handle 64 bits
with only its low 32 significant (Windows keeps USER / GDI / kernel handles
within 32 bits for exactly this: `GetDC`, `LoadCursorFromFile`,
`Form.Handle` fit a LONG).

- Every argument of a numeric type goes in a 64-bit integer slot (RCX, RDX,
  R8, R9 then the stack on x64; X0–X7 then the stack on ARM64 — the one
  convention 64-bit Windows has; `extern "system"` is it). A LONG's value is
  sign-extended, so `-1` is `HWND_TOPMOST` and `INFINITE` alike; WORD /
  SHORT / BYTE are masked to their size. `DOUBLE` goes in a float register,
  `SINGLE` as its 32 bits in one (GDI+'s `GdipDrawLine(g, pen, x1 AS
  SINGLE, …)`). Up to 16 arguments, of which up to 8 DOUBLE / SINGLE (no C
  library, no assembler: `rapidr_runtime_core::ffi` is a table of exact
  `extern "system"` signatures — on x64 the first four arguments' kinds
  pick the signature and the rest go on the stack by position; on ARM64,
  and on the other systems' x64, integers and floats fill registers of
  their own, so one signature takes the integers, then the floats). More
  than that, a CURRENCY by value, a string given for a float, or a count
  of arguments other than the DECLARE's is a clear run-time error.
- The result: a 32-bit declared type is taken from the low 32 bits of the
  register (the upper ones are undefined for such a function) — LONG /
  INTEGER / DWORD sign-extended (RapidQ's DWORD is signed), WORD / SHORT /
  BYTE as their size; `DOUBLE` from the float register, `SINGLE` its low 32
  bits; `STRING` the C string at the returned address (read up to its NUL,
  at most 1 MB); `INT64` the whole register.
- A 64-bit pointer where the DECLARE says LONG — a result (`GlobalAlloc`,
  `GetProcAddress`, a DLL's `HMODULE`) or what a DLL writes into a BYREF
  LONG (`AVIFileOpen(pfile, …)`, `GetModuleHandleEx(…, hModule)`) — doesn't
  fit 32 bits. RapidR keeps it and gives the program a 32-bit *stand-in*
  (from 0xD1E00000, a pattern no flag combination of Windows' makes), which
  turns back into the pointer when the program hands it to a DLL again.
  Only an address the process has mapped gets one (`VirtualQuery`), so a
  32-bit result over a register's stale upper half stays itself. PEEK,
  POKE and MEMCPY on a stand-in are refused by name: it is the DLL's
  memory, not the program's.
- Loading: `libloading` (ISC / MIT) with Windows' own search (`"user32"`,
  `"user32.dll"`, a path). A 32-bit DLL (one shipped with an old example)
  can't load into a 64-bit program: RapidR reads the DLL's PE header and
  says so ("'PASCAL.DLL' is a 32-bit DLL; RapidR programs are 64-bit, so
  Windows can't load it"); a DLL that isn't there is "can't find the DLL".
  A 32-bit DLL that only holds resources, loaded with `LoadLibrary` (the
  cursor example's CURSORS.DLL), opens as data, so `LoadCursor(hInst, …)`
  finds its cursors.
- A crash inside the DLL (an argument that isn't a valid pointer, as RapidQ
  crashes too) ends the program with `run-time error: the call to X in
  Y.dll crashed (access violation …)` through an unhandled-exception
  filter, instead of vanishing.
- `CODEPTR` / `CALLBACK` handed to a DLL (window procedures, enumeration
  callbacks) isn't supported yet: a clear run-time error before the call.
- x86 machine code a program wrote into a string or a buffer and runs with
  `CallWindowProc(VARPTR(code$), …)` (RapidQ's way to run assembler) can't
  run in a 64-bit program: a clear run-time error before Windows is called.
- RapidQ's own QRECT given to a DLL (`GetClientRect(hWnd, r AS QRECT)`,
  `include/qfocus.inc`, `commctrl32.inc`) is a RECT of its four LONGs —
  the same on 64-bit Windows — and what the DLL wrote is stored back into
  it (before, the record's name went over as a string and the API wrote
  into a copy). A QNOTIFYICONDATA has 64-bit handles in Windows' 64-bit
  NOTIFYICONDATA, so handing one to a DLL is refused with an error that
  says so; `Shell_NotifyIcon` itself is RapidR's tray on every system
  (`rapidr_value::tray`).
- A TYPE is passed with RapidQ's packed 32-bit layout. Structures whose
  fields Windows' 64-bit version widens — pointers and handles declared as
  LONG (`TCITEM.pszText`, `SECURITY_ATTRIBUTES.lpSecurityDescriptor`,
  `NOTIFYICONDATA.hWnd`) — don't match what the API reads; the call fails
  or crashes, named. Structures of numbers only (`RECT`, `POINT`,
  `SYSTEMTIME`, `LOGFONT`) work.
- `RAPIDR_SANDBOX` set (to anything but empty or 0): no library is loaded
  and every DLL call is a run-time error — the gate a run RapidR starts on
  someone else's behalf (an assistant's, an extension's) sets
  (docs/security-audit.md SEC-19).

RapidQ's own Windows-message built-ins are those functions of user32's:
`SENDMESSAGE hWnd, uMsg, wParam, lParam` is `SendMessageA`, `POSTMESSAGE`
`PostMessageA`, `KILLMESSAGE hWnd, uMsg` takes that message off the queue
(`PeekMessageA` with `PM_REMOVE`) — called on Windows, the error elsewhere,
as if the program had DECLAREd them (`rapidr_ast::memory`); a program that
DECLAREs or defines a routine of that name keeps its own.

### On macOS, Linux and the web

A call into a Windows system DLL (user32, kernel32, gdi32, shell32, winmm,
advapi32, comctl32, comdlg32, wsock32, ws2_32, ole32, odbc32, opengl32, …:
`rapidr_value::dll::is_windows_system_library`) is a run-time error:

```
run-time error: 'GetDC' is a Windows function (user32): this program calls Windows itself, so it runs on Windows only. For every system, draw with an RCANVAS (Line, Circle, Rectangle, TextOut, …)
```

The second sentence is RapidR's hint for that API family when it has one
(`rapidr_value::dll::windows_api_hint`: SHELL for ShellExecute, RSOCKET for
wsock32, RSQLITE for odbc32, TIMER for GetTickCount, …). The program
compiles on every system — the DECLARE is RapidQ, nothing is wrong with it
— and runs until it calls Windows: a program that only calls `Beep` from a
menu runs fine until that menu item. The web can't load any DLL: the same
error there, worded "the web can't load DLLs". A program's own `.dylib` /
`.so` on macOS / Linux (a RapidR extension, `LIB "mylib.dylib"`) still
loads through the same mechanism.

## 2. One address space: VARPTR, DLL pointers, PEEK / POKE

`rapidr_value::memory` keeps RapidQ's memory functions memory-safe on every
runtime (v2.56.0): an address is a number inside a *block*, and a block is
a live view of something the program owns — an array's elements, a TYPE's
packed fields, a stream's buffer, a *mirror* of a plain variable's bytes
(copied back into the variable after each statement that may write memory:
`rapidr_ast::memory`). Addresses run from 1 MB to 2 GB (`FIRST` …
`LIMIT`), so they fit a LONG and 0 is never valid. The block registry
(`Space.blocks`) is what PEEK / POKE validate against.

**The same addresses are the real ones on Windows.** When a DLL is called,
every live block is *materialised*: its bytes as RapidQ lays them out are
written into real memory **at the block's own address** (`memory::backing`:
the pages are reserved and committed with `VirtualAlloc` at that address —
64 KB-granular reservations below 2 GB, which a 64-bit process has free —
the first time a block is used in a call). So:

- an address the program got from `VARPTR` / `UDTPTR` / `Mem.Pointer`, or
  worked out from one (`VARPTR(a(0)) + 4 * 3`), is a valid pointer for the
  DLL — the buffer the API fills *is* the program's memory;
- a pointer **inside** a structure is valid too (a STRING field is stored
  as the address of its characters — `lpszClassName`, `BROWSEINFO.lpszTitle`
  — and that address is real as well);
- after the call every block whose bytes changed is decoded back into the
  array / TYPE / variable it views (`memory::read_back`), and the mirrors'
  variables are refreshed by the statement's `__mem_sync`.

The cost is the encoding of the live blocks per call, proportional to the
program's VARPTR'd data; a program with no addresses handed out pays
nothing. (Linux maps the pages the same way with `mmap` at the address;
macOS can't — its first 4 GB are the unmapped `__PAGEZERO` — so a `.dylib`
there gets each block in a buffer of its own, pointer identity kept per
block, pointers *inside* structures not translated. The web has no DLLs.)

**PEEK / POKE** (RapidQ: console functions, QBasic-compatible — checked
with RC.EXE on screen, `speek/s_peek`): `PEEK([#page,] address)` reads a
byte of a console *page*, `POKE [#page,] address, byte` writes one; page 0
is the screen (80 × 25, 4000 bytes: even addresses the character, odd the
attribute `background SHL 4 OR foreground`; a blank cell reads 32 and 7;
`CLS` fills it with the current colour's attribute; a POKE shows at once);
pages 1–7 are off-screen buffers that start as zeros, copied with
`PCOPY from, to`. RapidR keeps page 0 as a model of what the program
printed (`rapidr_value::console`: every PRINT, LOCATE, COLOR and CLS
updates it; a POKE draws the cell with escape sequences) — the same on the
desktop and the web's console — and the seven other pages as memory.

RapidR's addition, Robert's decision: an address at or above 1 MB is the
managed memory above — `PEEK(VARPTR(i))` reads the variable's first byte,
`POKE VARPTR(s$) + 1, 65` changes its second character, on every runtime
including the web (the memory model is shared, no real memory is involved).
`PEEK` of an address that is neither a console page's nor inside a live
block, and a page outside 0–7, is a run-time error that says so — never a
crash. (RapidQ reads whatever lies there: `PEEK(100000)` printed 0.)

**QMEMORYSTREAM's MemCopyFrom / MemCopyTo** (RC.EXE: `memcopy` probes,
`tests/conformance/cases/memstream_memcopy.bas`) are the same model seen
from a stream: `Mem.MemCopyFrom(addr, n)` writes n bytes from `addr` at
Position (a gap past the end zero-filled, nothing before the start) and
moves Position on; `Mem.MemCopyTo(addr, n)` copies n bytes from Position to
`addr` — zeros for bytes past the data, where RapidQ copied whatever
followed its buffer — and moves Position by n, past the end too, and back
for a negative n, which copies nothing (all as RC.EXE). `addr` must be the
program's own memory (VARPTR of a variable, an element, a TYPE, a
stream's Pointer); the count is checked against that block *before*
anything is copied, and the statement copies the variable's mirror back,
as MEMCPY's does. `SaveUDTArray` / `LoadUDTArray` (one argument, a TYPE's
array field: `Mem.SaveUDTArray(t.Items)`) write and read every element as
RapidQ lays them out; loading more than the stream holds is RapidQ's
"Stream read error". Where RapidQ raised an exception and ended (an address
it couldn't read, that read error) RapidR reports the error, as its object
methods do, and copies nothing.

Memory a DLL allocated (`GlobalLock`, `HeapAlloc`: a stand-in, §1) isn't
the program's, so PEEK, POKE, MEMCPY and MemCopyFrom / MemCopyTo refuse it
by name — Robert's decision: RapidR's own memory functions never touch
memory they can't check. A program reaches it the way a C program does,
through the DLL, as it declares: on Windows

```
DECLARE SUB CopyMemory LIB "kernel32" ALIAS "RtlMoveMemory" _
    (BYVAL Dest AS LONG, BYVAL Src AS LONG, BYVAL n AS LONG)
CopyMemory VARPTR(buffer$), p, 5     ' p: GlobalLock's pointer
```

copies between the DLL's memory and the program's (`VARPTR` is real there,
the stand-in turns back into the pointer). A program that DECLAREs a
routine named `RtlMoveMemory` gets that DLL routine; RapidR's own
`RTLMOVEMEMORY dest, src, n` (MEMCPY of the two variables, on every
system) is only for a program that doesn't declare it — before, the
rewrite also took declared ones and copied `BYVAL VARPTR(…)` arguments
between temporaries without a word.

`INP` / `OUT` / `INPW` / `OUTW` (hardware ports) are a run-time error on
every system — as on every Windows since 2000, where RapidQ's own INP
raises `EPrivilege` — not a compile error: the program runs up to that
statement, as it does under RapidQ.

## 3. Handles: `Form.Handle` is the window's HWND on Windows

RapidR draws its own controls in one window per form, so only a form has a
window of its own. RapidR's own handles (a control's, a form's before it is
shown, an icon's) are numbers no Windows USER handle can be: a USER handle
is an index into the session's handle table (its low word, the table at
most 65,536 entries for the whole session) and a reuse count (its high
word), and RapidR's have a low word near 0xFFFF (`handles::own_handle`) —
so `ShowWindow(Edit.Handle, …)` or `SendMessage(Button.Handle, WM_CLOSE, …)`
can't reach another program's window (RapidR's first numbering, 0x10004 +
4n, was the desktop window's range). On Windows, once a form is shown, `Form.Handle` is its
real HWND (the winit host registers it with `rapidr_value::handles` when
the window is made), so `SetWindowPos`, `SetForegroundWindow`, `GetDC` /
`ReleaseDC`, `GetWindowRect`, `SetClassLong(Form.Handle, GCL_HCURSOR,
LoadCursorFromFile(…))`, `FlashWindow`, `ShowWindow` work on forms. Before
the form is shown, and on the other systems, `Handle` is RapidR's own
number for the component (stable, never 0, `handles::name_of` finds the
component again for a QFORMMDI's `AddChild`).

What can't work, and says so by failing the Windows way (the API returns
0, `GetLastError` 1400 "invalid window handle"), never by crashing:

- **Per-control HWNDs**: `Edit.Handle`, `Button.Handle` are RapidR's
  numbers, not windows. `SendMessage(Edit.Handle, EM_SETSEL, …)`,
  `SetWindowLong(Button.Handle, GWL_STYLE, …)`, `GetWindowRect` of a
  control, `SetParent` of a control, `MoveWindow` on a control do nothing
  useful. Use the component's own properties and methods.
- **Subclassing**: `SetWindowLong(…, GWL_WNDPROC, CODEPTR(Proc))` and
  `CallWindowProc` need a callback into the program (not supported yet)
  and, for controls, a real HWND.
- **Messages the program expects to receive** (`WM_*` through a `WndProc`,
  `RegisterHotKey`'s `WM_HOTKEY`): the same.
- **Drawing on a form's DC** (`GetDC(Form.Handle)` + `Rectangle`) does draw
  on the window, but RapidR repaints the form from its own model on the
  next paint, so the drawing lasts until then. Draw on a QCANVAS instead.
- **Window regions**: `SetWindowRgn(Form.Handle, CreateEllipticRgn(…), 1)`
  succeeds and `GetWindowRgn` reads the region back, but the window isn't
  cut to it — RapidR draws its windows with the GPU, which Windows composes
  whole (RapidQ's rounded and elliptic forms, `forms/QrForm.bas`, stay
  rectangles). Open.
- **Coordinates on a high-DPI screen.** RapidR is DPI-aware; RapidQ wasn't
  (Windows scaled its windows and gave it scaled coordinates). So the
  numbers Windows' functions take and give — `SetWindowPos`,
  `GetWindowRect`, `CreateRoundRectRgn(0, 0, Form.Width, Form.Height, …)` —
  are the screen's pixels in RapidR, where `Form.Width` is in RapidQ's
  units: at 200% a form moved with `SetWindowPos(…, 120, 90, 420, 260, …)`
  is half the size it was under RapidQ. At 100% they agree.
- **A form's properties after an API moved it** follow once the program's
  handler returns: `SetWindowPos` then `Form.Left` in the same SUB reads
  the old value, a later event the new one.

### Cursors a program loads through Windows

`Screen.Cursors(i) = LoadCursorFromFile("x.cur")` (or `LoadCursor` of a
resource DLL's cursor) stores the HCURSOR as RapidQ's TScreen.Cursors does,
and a form or control with `Cursor = i` shows it: on Windows the winit host
makes a cursor of that HCURSOR's pixels and hot spot (`wincursor.rs`;
an `.ani` file shows its first frame), so a `.cur` looks as it does under
RapidQ — checked against RC.EXE's build in the VM (§6): a `.cur` shown over
a form, and `cursors/animated`, which loads twelve cursors from a resource
DLL and steps `Form.Cursor` through them on a timer. `Screen.Cursors(i) = 0` puts the standard one back. On the other
systems `LoadCursorFromFile` is a Windows function: the "Windows only"
error, as every DLL call.

## 4. The lexer: `&hHE`

RC.EXE reads a hex literal as the alphanumeric run after `&H` with the
non-hex letters dropped: `&hHE` is 14, `&hHA` 10, `&hG1` 1, `&hZ` and a
bare `&h` 0 (`hex/h1` … `h6`; RapidQ's `sound/SBLIB.BAS` has `&hHE`).
RapidR's lexer does the same. (`&O` / `&B` aren't RapidQ's: `&oO7` printed
0 there; RapidR keeps them as its own octal / binary literals.)

## 5. Where the pieces are

- `rapidr_value::dll` — the Windows DLL list, the hints, the error texts,
  the DECLARE's calling spec (`spec_of`, `Spec::parse`): shared by both
  compilers and every runtime.
- `rapidr_value::memory` — the block registry; `backing` (real pages at
  the block's address), `materialize` / `read_back` around a call;
  PEEK / POKE on managed memory.
- `rapidr_value::console` — the console pages, PEEK / POKE / PCOPY there.
- `rapidr_value::objects::stream_ops` — MemCopyFrom / MemCopyTo,
  SaveUDTArray / LoadUDTArray on the block registry; `rapidr_ast::stream_arrays`
  gives the UDT array field's type.
- `rapidr_runtime_core::ffi` — `dll_call(lib, alias, spec, args)`: loading,
  marshalling, the signature table, the crash filter. Native builds call
  it as `ffi::rp_dll_call`; the native VM host as the `__dll_call` builtin.
  The web host and `rapidr_runtime_web` answer `__dll_call` with the error.
- `rapidr_ast::memory` — a call to a DLL routine has its STRING and BYREF
  arguments that are variables turned into mirrors (`VARPTR`), refreshed
  before and copied back after the statement, as `MEMCPY`'s are.
- Both compilers lower a call to a DECLAREd LIB routine to
  `__dll_call(lib, alias, spec, args…)`; nothing about DLLs is refused at
  compile time any more.
- `rapidr-ui-host-winit` registers each form window's HWND.
- Conformance: `peek_poke_memory`, `peek_poke_pages`, `peek_bad_address`
  (`.expected-runtime-error`), `inp_port_error`, `dll_needs_windows`
  (skipped on Windows by its `' skip-on: win32` marker, where the call
  succeeds), `dll_missing_library`, `sendmessage_needs_windows`,
  `sendmessage_declared`, `dotted_routine_calls`, `dll_rtlmovememory_declared`,
  `memstream_memcopy`, `stream_udt_arrays`, `filestream_bytes` (the
  streams' expected output is RC.EXE's); unit tests in
  `rapidr_runtime_core::ffi` (the C library called with integers, doubles,
  SINGLEs, 11-argument mixes, a pointer written into a BYREF LONG) run on
  macOS and in the Windows 11 VM; `tests/security/peek_poke_unowned_memory.mjs`
  (SEC-19).

## 6. RapidQ's corpus on Windows (2026-10-08)

The 175 programs of RapidQ's examples that declare DLL routines (in their
own source or an include they pull in) were built and run in the Windows
11 VM (ARM64), interpreted and as native builds, each for three seconds
with its windows captured; programs that would print, write the registry,
install fonts, start other programs, inject keystrokes or shut the machine
down were only built. The same set ran interpreted on macOS. The table of
every program is in [windows-dll-corpus.md](windows-dll-corpus.md);
`tools/dll_breadth.py` reruns it (the list: `tools/dll_corpus_list.py`).

| | before this pass | after |
|---|---|---|
| compile | 115 | 126 (SENDMESSAGE / POSTMESSAGE / KILLMESSAGE) |
| run 3 s without an error, interpreted | 73 | 81 |
| run 3 s without an error, native | — | 77 (6 more don't build natively: codegen gaps that aren't about DLLs; two checked with the build from before this pass, which fails the same way) |
| only built (would print, touch the system) | 22 | 24 |
| macOS | | every run that reaches a DLL call stops with the "runs on Windows only" error; none crashes |

What still stops the others, most common first:

1. **Callbacks** — a SUB handed to Windows to call back (`SetWindowLong(…,
   GWL_WNDPROC, CODEPTR(WndProc))`, a window class of the program's own,
   RapidQ's `CallBack_4.inc` forwarders): `MinToTaskbar`, `ColorButton`,
   `titlebtn`, `NewWndProcDemo`, `TestNoMouse`, `ManageGridEditing`,
   `purewindows/wnd` — 15 corpus programs use `CODEPTR`, most of them for
   a DLL.
   Needs C → RapidR trampolines and a way for the interpreter to run a
   handler while Windows waits for its result (§1; ROADMAP).
2. **x86 machine code** run through `CallWindowProc` (RapidQ's assembler
   trick): `ReverseStringDemo_1`, `TestRqAsmUtils`, `GetDllsFuncList`,
   `AlphaTest2` — can't run in a 64-bit program; a clear error.
3. **DLLs that aren't there or are 32-bit** — see below.
4. **TYPEs with pointer or handle fields** declared LONG, which 64-bit
   Windows reads 8 bytes wide: `QTabVertical` (TCITEM), `Pipe`
   (SECURITY_ATTRIBUTES), `testAVItoBMP` (a PAVIFILE written into a TYPE
   field). A layout translation (the fields Windows widens, by the SDK's
   names) would make these work; it isn't RapidQ's own behaviour to invent,
   so it waits for a decision.
5. Four programs wait longer than the three seconds (two sound programs, a
   browser-window resizer, the own-window-class one above), one stops on an
   unrelated object error (`jpeg_Blend`).

The compile failures that remain aren't about DLLs: 10 includes the corpus
doesn't have, 7 `QRECT` fields in a TYPE (RC.EXE's own "Datatype %s not
supported in STRUCT" — those DirectX programs don't compile under RapidQ
either), and code that isn't RapidQ's (other BASICs' syntax, typos).

### Side by side with RC.EXE (user32, kernel32, gdi32)

The same programs built by RapidQ's RC.EXE and by RapidR (interpreted and
native) and run in turn in the VM, their windows captured (2026-10-08,
second pass; the probes are in the lane's scratch folder, not the
repository):

| Program | Windows API | What RC.EXE and RapidR show |
|---|---|---|
| `files/DiskVolumeInfo.bas` | GetVolumeInformation (kernel32), buffers by VARPTR, a BYREF serial | the same three message boxes: no volume name, NTFS, serial 653868438 (26F9-3D96) |
| `files/GetShortFileName.bas` | GetShortPathName (kernel32), a STRING and a VARPTR buffer | `C:\PROGRA~3\MICROS~1` on all three |
| `System/SPYINFO3A.bas` | GetCursorPos, WindowFromPoint, GetWindowText, GetClassName, GetWindowRect, GetParent, SetWindowPos, SetWindowLong, GetSysColor (user32) | the window under the mouse, its class, parent and rectangle — real data in both; RapidR's in physical pixels (it is DPI-aware, §3). Its "Lock to" check box stays below the form in RapidR: the program places it in OnResize, which RapidQ fires when a form shows (RC.EXE: Resize, Show, Resize, Paint) and RapidR doesn't yet |
| `forms/AlwaysOnTop.bas` | SetWindowPos, GetActiveWindow, GetForegroundWindow | the same form and menu |
| `forms/LockWin.bas` | LockWindowUpdate | RapidR fills the 25,001 items in half a second; RC.EXE, emulated on ARM, hadn't finished after a minute |
| `cursors/animated/cursors.bas` | LoadLibrary (a 32-bit resource DLL), LoadCursor | the DLL's twelve cursors stepping over the form, alike |
| a `.cur` through LoadCursorFromFile into `Screen.Cursors(1)`, `Cursor = 1` | LoadCursorFromFile | the file's cursor over the form, alike (RC.EXE, interpreted, native) |
| a centred form, GetClientRect into a QRECT | GetClientRect, GetWindowRect | the form at 714, 401 in both (fixed in this pass: RapidR opened centred forms at 0, 0); the QRECT filled (fixed: it went as a string) |
| `forms/QrForm.bas`, `forms/Qeform.bas` | CreateRoundRectRgn / CreateEllipticRgn (gdi32), SetWindowRgn | the calls succeed in RapidR too (GetWindowRgn reads the region back), but its window isn't cut to the region: RapidR draws a window with the GPU, which Windows composes whole; RC.EXE's window is rounded / elliptic. Open |
| `graphics/GDI_p.bas` | GdiplusStartup (gdiplus) | RC.EXE: started. RapidR: error 17 (UnsupportedGdiplusVersion). The program passes `VARPTR(tSI)` BYREF — the address of a LONG holding the TYPE's address — where GDI+ wants the TYPE; under RapidQ `VARPTR` of a TYPE isn't an address (RC.EXE: 0 or a stray value — 1 here, which GDI+ reads as its version), so it works there by accident. Its drawing also wants a QBITMAP's Handle as an HDC, which RapidR's isn't (§3) |
| `graphics/bitblt.bas` | GetDC, BitBlt | neither: the corpus lacks its `rq.bmp` |
| memory a DLL allocated (GlobalAlloc / GlobalLock) through a DECLAREd RtlMoveMemory | kernel32 | `[Hello]` in both; `Mem.MemCopyFrom` of that pointer copies in RapidQ and is refused in RapidR (§2) |

**32-bit DLLs** a RapidQ program ships, which no 64-bit program can load
(RapidR says "… is a 32-bit DLL"): `PASCAL.DLL` (`dll/simple/DLL.BAS`),
`STKIT432.DLL` (`System/CreateShellLink.bas`, `reminder/reminder.bas`),
`IO.DLL` (`io/IO_RQ.bas`, which names it `c:\rapid-q\IO.DLL`),
`NVIEWLIB.DLL` (`graphics/Picview.bas`, `QHTML` ×4), `JPEG.DLL`
(`graphics/LoadJPG.bas`), `RqUtils.dll` (`asm/TestRqUtilsDll.Bas`),
`sqlite3.dll` / `zlibwapi.dll` / `hexconvert` (`Database/SQL_blobs`,
`zlib/z_test.bas`, `zlib/zunzip.bas`), `FileSearch_FB_DLL`
(`files/FileSearch.bas`), `d3drm.dll` (8 Direct3D programs: Direct3D
Retained Mode, which left Windows with Vista — RapidR's own QD3D objects
replace it). `RAPIDQ32.DLL` (`Compile/rapidq_boosta.bas`) isn't in the
corpus at all. A 64-bit build of each would load; for most RapidR has a
component of its own (RSQLITE, RIMAGE, the zip functions). A 32-bit DLL
that only holds resources (`cursors/animated/CURSORS.DLL`) works: it is
opened as data for its cursors.

**The console.** `console/3dbox/3DBOX.BAS` (CLS, COLOR, LOCATE, a dotted
SUB name, POKE into the screen's last attribute) draws the same boxes,
colours and status line as RC.EXE's build on Windows, interpreted and
native, on macOS and on the web. One difference remains: RapidQ's console
shows bytes 128–255 in the console's DOS code page (437: `┌─┐│└┘`), where
RapidR shows them as Latin-1 (`ÚÄ¿³ÀÙ`), as a RapidQ program's strings
are read. Mapping console output through code page 437 would match RapidQ
for box-drawing programs and change accented text in console programs;
that choice is open.

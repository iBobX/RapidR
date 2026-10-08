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
  R8, R9 then the stack on x64; X0–X7 on ARM64 — the one convention 64-bit
  Windows has; `extern "system"` is it). A LONG's value is sign-extended, so
  `-1` is `HWND_TOPMOST` and `INFINITE` alike; WORD / SHORT / BYTE are
  masked to their size. `DOUBLE` goes in a float register (`SINGLE` as a
  32-bit float). Up to 16 integer arguments; DOUBLE arguments in calls of
  up to 4 arguments (what RapidQ's own DLL users need; no C library, no
  assembler: `rapidr_runtime_core::ffi` is a table of exact `extern
  "system"` signatures).
- The result: a 32-bit declared type is taken from the low 32 bits of the
  register (the upper ones are undefined for such a function) — LONG /
  INTEGER / DWORD sign-extended (RapidQ's DWORD is signed), WORD / SHORT /
  BYTE as their size; `DOUBLE` from the float register; `STRING` the C
  string at the returned address; `INT64` the whole register.
- A 64-bit pointer a DLL returns (`GlobalAlloc`, `GetProcAddress`, a DLL's
  `HMODULE`) doesn't fit a LONG; stored into one it becomes -2147483648 as
  any out-of-range store does (RapidQ's rule). Declare such results `AS
  INT64` (a RapidR type) or pass them straight on.
- Loading: `libloading` (ISC / MIT) with Windows' own search (`"user32"`,
  `"user32.dll"`, a path). A 32-bit DLL (one shipped with an old example)
  can't load into a 64-bit program: the error says so ("… is a 32-bit DLL;
  RapidR programs are 64-bit, so Windows can't load it").
- A crash inside the DLL (an argument that isn't a valid pointer, as RapidQ
  crashes too) ends the program with `run-time error: the call to X in
  Y.dll crashed (access violation …)` through an unhandled-exception
  filter, instead of vanishing.
- `CODEPTR` / `CALLBACK` handed to a DLL (window procedures, enumeration
  callbacks) isn't supported yet: a clear run-time error.

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

`INP` / `OUT` / `INPW` / `OUTW` (hardware ports) are a run-time error on
every system — as on every Windows since 2000, where RapidQ's own INP
raises `EPrivilege` — not a compile error: the program runs up to that
statement, as it does under RapidQ.

## 3. Handles: `Form.Handle` is the window's HWND on Windows

RapidR draws its own controls in one window per form, so only a form has a
window of its own. On Windows, once a form is shown, `Form.Handle` is its
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
  succeeds), `dll_missing_library`.

# RFormEx: design

A form with more: files dropped onto it one by one, its own icon in the
system tray (with animation), the whole window see-through or only its
background, always on top, told when it is minimised, when it gets and loses
the focus, the mouse wheel, and (on Windows) the window's messages. One
component, `RFormEx`, RapidR's own and native on every system the OS allows,
AI-first (every member described in the registry, for RAI and MCP alike).

Why: a common RapidQ community include, Rapidq2.inc, defines `TYPE QFormEx
EXTENDS QFORM` with these features. It reaches them by writing 32-bit x86
machine code into memory and making it the window's procedure. RapidR never
runs code a program writes into memory (memory safety, 64-bit, every
system), so every program that uses that QFormEx stops when its form shows:
a reported program that uses Rapidq2.inc's QFormEx to hear MIDI input
(winmm's MM_MIM_DATA through `OnMessage`) never gets past it. RFormEx gives
those programs the same members, natively, and the compatibility layer (§6)
makes `QFormEx` mean `RFormEx`, the same way `QMIDI` (QMidi.inc) means
`RMIDI`.

## 1. Principles

- **One form, more members.** At run time an RFormEx *is* an RForm (the same
  store type, `RFORM`, registered as an RFormEx by `rapidr_value::formex`, as
  `rapidr_value::mdi` registers an RFormMDI). Every RForm member, event and
  look is the RForm's, unchanged; nothing an RForm program does changes.
- **The registry knows it as its own component.** `RFORMEX` (display
  `FormEx`, RapidQ name `QFORMEX`, `from = "RAPIDQ2.INC"`, `extends =
  "RFORM"`: the build takes RForm's members and adds RFormEx's), so the
  toolbox, the inspector, completion, Help, the manual's reference, RAI and
  MCP all see RForm's members plus these.
- **One shared model, thin hosts.** What an RFormEx is (its tray icon, its
  drop zone, which messages OnMessage hears, the opacity it wants) lives in
  `rapidr_value::formex`; the desktop host and the web host only carry out
  window commands and report what the window system did.
- **Behaviour in our own words.** The include's code is never copied; this
  document and the registry describe what its members do. Where the include
  behaves in a way that only makes a member unusable, RFormEx picks the
  useful meaning and says so (§7, decisions).

## 2. The component

`CREATE Main AS RFormEx … END CREATE` (or `DIM Main AS RFormEx`). Everything
of RForm, plus:

### Dropped files

| Member | What it is |
|---|---|
| `DragEnable` (bool, False) | True: files dragged from the computer (Finder, File Explorer, a file manager) can be dropped on the form; OnDrag hears each one. False: the form doesn't take them (RForm's OnDropFiles still does, as on any form). |
| `DragZone` (rect: `Left`, `Top`, `Right`, `Bottom`, client pixels) | Only drops inside it count. Empty (Right ≤ Left or Bottom ≤ Top, as it starts): the whole form (§7 D1). |
| `DropPoint` (point: `X`, `Y`, read) | Where the last drop landed, in the form's client pixels; set before OnDrag. |
| `OnDrag(File AS STRING)` | One file dropped: once per file, in the order the system gave them, with its whole path. In a browser the file is read into the program's own files and `File` is its name. |

### The tray icon

| Member | What it is |
|---|---|
| `AddTrayIcon` | Puts the form's icon in the system tray: the notification area on Windows, the menu bar on macOS, the panel's tray on Linux (StatusNotifierItem), a strip in the page on the web. The picture is `TrayICO`'s current one, else the form's / application's icon, else RapidR's. |
| `DelTrayIcon` | Takes it away. Closing the form for good, or the program ending, takes it away too. |
| `ModifyTrayIcon` | Shows the icon again with the current picture and hint (setting `TrayIconHint` or `TrayIconIndex` does it already). |
| `TrayICO` (an RImageList) | The pictures: `Main.TrayICO.AddICOFile "a.ico"`, `AddBMPFile`, `AddICOHandle`, `AddSVGFile` … |
| `TrayIconHint` (string, the application's title) | The tooltip. |
| `TrayIconIndex` (int) | Which of TrayICO's pictures shows, from 1. |
| `TrayIconUpdate` (int, ms, 2000) | With two pictures or more, they take turns at this pace while the icon is shown (an animation); 0 stops it. |
| `TrayIconShown` (bool, read; RapidR's) | True while the icon is in the tray. |
| `OnTrayClick` | The right mouse button was released on the icon (on macOS also a Control-click): the place to show a pop-up menu. |
| `OnTrayDblClick` | The icon was double-clicked with the left button: the place to show the form again. |
| `OnTrayMouseUp(Button AS INTEGER)` (RapidR's) | Any button released on the icon: `mbLeft` 0, `mbRight` 1, `mbMiddle` 2, for a program that wants a left click too. |

### The window

| Member | What it is |
|---|---|
| `FormStyle` | `fsStayOnTop`: the window stays above other programs' windows (Windows, macOS, X11; on Wayland the compositor decides; in a browser above the page's other windows). `fsNormal` (0) is an ordinary window again. An RForm's FormStyle does the same (§7 D5). |
| `Transparent` (int 0–255, 255) | The whole window — frame, title bar, controls — see-through: 255 opaque, 127 half, 0 invisible (it still takes the mouse). |
| `TransparentClient` (bool, False) | The form's background colour (`Color`) becomes a hole: the desktop shows through it and clicks there go to what is behind. Controls stay, except their parts in exactly that colour. |
| `RemoveTaskBar` | Windows: the window's button leaves the taskbar (a tray program). Elsewhere nothing: macOS shows the application in the Dock, not its windows. |
| `OnMinimise` | The person minimised the window (its button, the system menu, the taskbar or Dock): not when the program sets `WindowState` itself. A tray program hides the form here. |
| `OnGetFocus` | The window became the active one (clicked, Alt+Tab / Command+Tab, the program's own SetFocus). |
| `OnLostFocus` | It stopped being the active one (another window or program took the keyboard). |
| `OnMouseWheel(Delta, X, Y, Shift)` | RForm's own (unchanged): notches, the mouse in the form's pixels. |
| `OnMessage(Hwnd, Msg, wParam, lParam)` | The window's messages (§4). |
| `InheritOnShow`, `InheritOnClose` | Accepted and do nothing: the include's own OnShow / OnClose work (hooking the window, the animation, the tray icon's removal) is RFormEx's own (§6). |

## 3. The shared model: `rapidr_value::formex`

- `register(form)` / `is(form)`: which forms are RFormExes (runtime-core's
  and the web runtime's create make an `RFORM` and register it).
- The **drop zone** test (`accepts(form, x, y)`) and the per-file
  `OnDrag` list from one drop (`drops(form, files, point)`).
- The **tray**: the form's own icon in `rapidr_value::tray` as (the form's
  Handle, uID `0x5246`, message `0x590`, the hint, the picture of TrayICO
  item `TrayIconIndex` by its icon handle); `tray_event(lparam)` turns the
  icon's mouse messages into `OnTrayClick` (WM_RBUTTONUP), `OnTrayDblClick`
  (WM_LBUTTONDBLCLK) and `OnTrayMouseUp` (any WM_xBUTTONUP); the animation's
  next picture (`tick`), paced by the runtime's timer list.
- **Which window messages OnMessage hears** (`heard(msg)`, §4).
- The **window wishes**: opacity (`Transparent`), colour key
  (`TransparentClient` + the form's resolved colour), on top, off the
  taskbar — turned into window commands (`WindowOp::Opacity`, `ColorKey`,
  `OnTop`, `Taskbar`, rapidr-ui-app) when they change.

## 4. OnMessage

**On Windows**: every message Windows sends or posts to the form's window,
through RapidR's own subclass (`rapidr_ui_host_winit::winmsg`, RForm's
WndProc path): MM_MIM_DATA from a MIDI input opened with CALLBACK_WINDOW,
WM_USER + n a program posts, WM_SIZE, the mouse and the keys … — except the
ones RFormEx turns into its own events, which go to those events instead:
WM_DROPFILES, WM_ACTIVATE, WM_KILLFOCUS, WM_SYSCOMMAND, WM_MOUSEWHEEL and
the tray icon's message (&H590). (That is the include's own split: a program
written for it never saw those in OnMessage.) The handler runs just after
Windows handled the message, in the order they came; its result is ignored.
RForm's `WndProc` stays the whole stream (an RFormEx has both).

**On macOS, Linux and the web** there are no Windows messages. OnMessage
hears exactly these, as Windows would number them:

- the system tray icons the program made itself with Shell_NotifyIcon
  (RNotifyIconData) and a `uCallbackMessage` (as RForm's WndProc does);
- `WM_SIZE` (&H5) when the window's size changes: wParam 0 restored,
  1 minimised, 2 maximised; lParam the client width + height × 65536;
- `WM_MOVE` (&H3) when it moves: lParam the client area's left + top ×
  65536 on the screen (16 bits each, signed).

Nothing else; the Help says so.

## 5. Per system

| Feature | Windows | macOS | Linux X11 | Linux Wayland | Web |
|---|---|---|---|---|---|
| OnDrag / DropPoint | winit's drop + the mouse position | same | same | same (position: last known pointer) | the page's drop (file read in) |
| Tray | notification area | menu bar item | StatusNotifierItem | same | strip in the page |
| Transparent | layered window (alpha) | window alpha | `_NET_WM_WINDOW_OPACITY` | not available (said once) | CSS opacity |
| TransparentClient | layered window colour key | window mask from the frame | window shape from the frame | not available (said once) | the colour's pixels cleared in the page |
| fsStayOnTop | always-on-top level | floating level | above state | compositor's choice | above the page's windows |
| RemoveTaskBar | skip taskbar | nothing | nothing | nothing | nothing |
| OnMinimise / OnGetFocus / OnLostFocus | the window system's events | same | same | same | the page's window frames |
| OnMessage | every message (§4) | tray + WM_SIZE + WM_MOVE | same | same | same |

A layered window (Windows' Transparent / TransparentClient) is drawn on the
CPU (vello_cpu + softbuffer, GDI's presentation, which layered windows
support everywhere); macOS' and X11's colour key read the CPU frame too. The
other forms keep the GPU.

## 6. The compatibility layer (QFormEx)

- `QFORMEX` → `RFORMEX` by the registry's RapidQ name (the parser's
  `canonical_type_name`), as `QFORMMDI` → `RFORMMDI`.
- The include's `TYPE QFormEx EXTENDS QFORM` is recognised by its **shape**,
  not the file's name (`rapidr_ast::formex`): named QFormEx, extending
  QFORM, and every public member (field, PROPERTY SET, method, EVENT field
  with its parameter count) one RFormEx knows (the table in formex.rs). Then
  the TYPE is left out (its fields and code are never compiled: the
  include's other components and functions stay as they are), and `CREATE
  Main AS QFormEx` / `DIM … AS QFormEx` / `EXTENDS QFormEx` make an RFormEx.
- A QFormEx TYPE of another shape (an older or newer include, or a
  program's own class of that name) is not guessed at: the compiler stops
  at the TYPE with `This QFormEx isn't the one RapidR knows: <member> …
  RapidR has its own RFormEx under that name …` and says what to do (use
  RFormEx's members, or name the TYPE otherwise).
- The parser records which members a TYPE declares under `PRIVATE:` (a
  list on TypeStatement) so the shape is the public interface only.

## 7. Decisions

One rule decides D1 to D4 (approved 2026-11-06): extensions are additive and
never change old behaviour. **A form written `QFormEx` behaves exactly as the
include's; a form written `RFormEx` gets RapidR's behaviour.** Same object
underneath: a per-instance "classic" flag (`rapidr_value::formex::is_classic`)
set by how it was created — `rapidr_ast::formex` adds `__classic = 1` as the
first statement of a `CREATE … AS QFormEx` (and after a `DIM … AS QFormEx`,
and first in the constructor of a `TYPE … EXTENDS QFormEx`); the parser keeps
the name `QFORMEX` as written for that pass.

| | Written `QFormEx` (the include's) | Written `RFormEx` (RapidR's) |
|---|---|---|
| D1 an empty `DragZone` (as it starts) | takes no drop | the whole form |
| D2 `Transparent` before it is set | reads 0; the window stays as it is until set | reads 255 (opaque) |
| D3 `OnMouseWheel` | 1 / -1 each time, the screen position (as the include worked it out from lParam), Windows' MK_SHIFT 4 / MK_CONTROL 8 | RForm's: whole notches, form pixels, RapidR's Shift |
| D4 `OnMinimise` | on Windows before the minimise (the host holds the window's SC_MINIMIZE, fires the event, then minimises); elsewhere once the system says it happened | once the window is minimised |

- **D5 fsStayOnTop puts any form's window on top.** RC.EXE settled it in
  the Windows VM (2026-11-06): a plain QFORM with `FormStyle = fsStayOnTop`,
  in its CREATE or after Show, has WS_EX_TOPMOST — RapidR's RForm ignored it
  (a code comment took RapidQ's FormStyle as fsNormal only). RForm now does
  as RC.EXE's QFORM, and RFormEx the same, both names. (The same probe read
  the include's QFormEx before anything is set: Transparent 0, TrayIconUpdate
  2000, DragZone 0,0,0,0, FormStyle 0, TrayIconIndex 0, DragEnable 0,
  TrayIconHint the program's title — the classic flavour's values.)
- **D6 OnTrayClick is the right button's release**, as the include;
  RapidR's OnTrayMouseUp(Button) adds the others.
- **D7 The tray icon goes when the form closes for good** (the include's own
  OnClose did it, unless the program replaced OnClose).

## 8. Tests

- `rapidr_value::formex` unit tests: the zone, the per-file drops, the
  message filter, the tray's mouse messages, the animation's order.
- `rapidr_ast::formex`: Rapidq2.inc's shape accepted (a fixture written for
  RapidR with the same public interface, never the include's code), a
  different shape refused with the message, the TYPE left out.
- Conformance (`tests/conformance/cases/formex_*.rr`, interpreter + native):
  defaults, DragZone / DropPoint, TrayIconShown after Add/Del, FormStyle.
- GUI events (`tests/native_gui_events.mjs`, interpreted + native, and the
  web flows): a drop (`RAPIDR_TEST_DROP` at a point) firing OnDrag per file
  inside / outside the zone, the tray's clicks (`tray.__rclick`…), focus,
  minimise, captures of Transparent and TransparentClient (macOS, the
  Windows VM, the Ubuntu VM).
- The QFormEx case: a program including the fixture include, CREATE AS
  QFormEx, OnMessage bound, InheritOnShow called, runs everywhere; and the
  reported MIDI program as written, on Windows with the stand-in winmm
  driver (tests/fixtures/fake_winmm), hearing notes and SysEx.

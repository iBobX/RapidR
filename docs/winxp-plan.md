# RapidR on Windows XP (the map, the plan, the numbers)

Lane WINXP, 2026-11-05. Background: a reported XP failure (below), the Windows 7
build in `tools/win7/` and `docs/old-pcs-spike.md`. **Windows XP is unofficial.** This is the honest
feasibility study the brief asked for, plus what has been built and seen running so far.

RapidR's mission reaches people on old PCs. RapidR Studio runs on Windows XP SP3 with
One-Core-API, but F5 fails there with "os error 123". Robert gave this lane a day to (A) fix F5 on
XP + One-Core-API, and (B) find out, honestly, whether a RapidR console program can run on **plain**
Windows XP SP3 with no One-Core-API, and if the cost is about a day, do it — console, then GUI, then
Build (interpreted).

## 0. The rule that governs everything here

Every Windows-XP-specific line lives **only** in the XP build flavour (`tools/winxp/build.sh`) and is
compiled into nothing else. The Windows 7/8, Windows 10/11, macOS, Linux and web binaries are
byte-for-byte what they would be without this lane. No source file, `Cargo.toml`, `RUSTFLAGS` or build
script of the normal build is touched; no toolchain change outside the XP and Windows 7 flavours. The
evidence is in [§7](#7-what-this-lane-cannot-affect-the-isolation-evidence). A shared-code fix (the
os error 123 path) would have to be correct on every Windows version — but it turned out the cause is
not shared code at all ([§2](#2-step-a-the-os-error-123-on-f5)).

## 1. Why plain XP refuses `rapidr.exe`

Two reasons, both at load time, before any code of ours runs:

1. **The PE header.** The Windows 7 build's header says operating-system and subsystem version 6.0;
   plain XP refuses an image above 5.1 ("not a valid Win32 application"). The XP flavour writes 5.1
   (`XP_HEADER=5.1`), which is a linker flag only — it changes nothing about the code.
2. **The import table.** XP's loader resolves every imported function by name before the program
   starts; one function XP's DLLs do not export, or one DLL XP does not have, and it refuses the whole
   `.exe`. The Windows 7 `rapidr.exe` imports **637 functions from 27 DLLs**; **85** of them are
   missing on plain Windows XP SP3 (the map below). So the XP flavour must answer for all 85 in its
   own object, or XP will not load the program — there is no "load the console part only".

## 2. Step A: the os error 123 on F5

The F5 failure reported on XP + One-Core-API: *"c:\R\rapidr.exe: The filename, directory name, or volume label
syntax is incorrect. (os error 123)"*. The brief guessed a forward-slash or a `\\?\` verbatim path in
Studio's spawn. It is **not** a path bug.

RapidR Studio runs a program as a child process with its output captured
(`crates/rapidr-session/src/process.rs`: `rapidr run --session <program>`, stdin/stdout/stderr piped).
Rust's standard library makes those pipes with two NT calls
(`library/std/src/sys/process/windows/child_pipe.rs`): `NtCreateNamedPipeFile` with an **empty** name
under the pipe file system's root (an unnamed pipe), then `NtOpenFile` with an empty name under that
pipe (its other end). Windows Vista, 7 and later accept an empty name there; **Windows XP's pipe file
system does not**, and answers `STATUS_OBJECT_NAME_INVALID`, which the standard library reports as
os error 123. So **every** program Studio launches with captured output fails on XP: F5, Build's
compile step, `SHELL` with output. The path strings are fine.

Nothing in Studio's own code is wrong, and changing Studio's spawn would not help (the failure is one
layer down, in `std`). The fix is an XP-flavour stand-in for those two `ntdll` functions
(`tools/winxp/shim/xp_pipes.c`): the real functions first (so XP + One-Core-API and every newer Windows
are unchanged); only when Windows refuses the unnamed pipe does it make the **same** pipe with a random
name under the same root and a security descriptor that lets only this user open it, remember
`(handle, name)`, and have the matching empty-name `NtOpenFile` open it by that name. On Windows 7 in
the VM, forced down the named path, a real child process's output comes back correctly
([§6](#6-what-has-been-seen-running)).

## 3. The map: what plain Windows XP SP3 lacks (85 imports)

`tools/winxp/imports.sh <exe>` lists an executable's imports and flags the ones missing on plain XP,
from `tools/winxp/xp-missing.txt` (built from Microsoft Learn's "minimum supported client" for each
function, and the DLLs XP ships — public documentation, never a DLL's own export table). Who pulls each
one in (`tools/winxp/` analysis, `llvm-nm` on the built libraries):

| Group | Count | Functions (how the XP flavour answers) |
|---|---|---|
| **Rust `std`** | 24 | SRW locks + condition variables (`Mutex`/`Condvar`), `InitOnce` (thread-local keys), FLS + `IsThreadAFiber` (thread-local destructors), `GetFileInformationByHandleEx` / `SetFileInformationByHandle` (metadata, `read_dir`, truncate, rename), `GetFinalPathNameByHandleW` (`canonicalize`), `CompareStringOrdinal`, the ProcThreadAttributeList trio, `CreateWaitableTimerExW`, `SetThreadStackGuarantee`, `CreateSymbolicLinkW`. Plus the pipe behaviour of §2. |
| **Toolchain (LLVM libunwind)** | 3 | `AcquireSRWLockShared`/`ReleaseSRWLockShared`, `K32EnumProcessModules` (backtrace). |
| **RapidR's own crates** | 6 | `rapidr-i18n`: `GetUserDefaultLocaleName`; `rapidr-ui-host-winit`: `RegGetValueW`, comctl32 window subclassing; `rapidr-value` (QREGISTRY through `windows-registry`): `RegCreateKeyTransactedW`, `RegOpenKeyTransactedW`, `RegDeleteTreeW`. |
| **Dependencies** | ~52 | `winit` (touch, DWM, display config, DPI, waitable timer), `wgpu-hal` (DXGI, display config), `accesskit` (UI Automation, `LocaleNameToLCID`), `windows-core` (19 PROPVARIANT/VARIANT conversions in `propsys`), `fontique` (DirectWrite), `rfd` (the Vista file dialog), `muda` (stock icon, subclass, DPI), `socket2` (`WSAPoll`, `WSASendMsg`), `windows-registry` (KTM: `CreateTransaction`, `CommitTransaction`), `schannel` (`NCryptFreeObject`), `tempfile` (`ReOpenFile`), `libloading` (`SetThreadErrorMode`), `chrono`/`jiff` (time zone), `dua`/`gix` (file info), `named_pipe` (`CancelIoEx`), `arboard` (final path). |

Whole DLLs XP does not have: `propsys`, `uiautomationcore`, `dwrite`, `dxgi`, `dwmapi`, `ktmw32`,
`ncrypt`.

## 4. How the XP flavour answers (and how it stays isolated)

The same mechanism as the Windows 7 build's C stand-ins (`tools/win7/clean-room.md`), extended to XP.
An object of our own C, in `tools/winxp/shim/`, is handed to the linker of the XP flavour's three
programs (`rapidr.exe` and the two runner stubs) and **nothing else**
(`cargo rustc ... -- -C link-arg=xp_*.o` over the unchanged Windows 7 libraries). For each of the 85
functions the object defines the three import names a caller can use — the import thunk `_Func@n` that
`std` calls, the slot `__imp__Func@n` that C's `dllimport` loads, and the undecorated `__imp_Func` of
the crates built with `--cfg windows_raw_dylib` (`tools/winxp/shim/xp_answer.h`). The linker takes an
object's own definitions before any import library's, so the DLL never enters the import table.

Each stand-in **first looks for Windows' own function** (`GetModuleHandleW` + `GetProcAddress`, loading
from Windows' own folder by full path if need be — `tools/winxp/shim/xp_common.h`). On Windows Vista, 7,
10, 11, and on XP with One-Core-API, the real one is found and called: behaviour is unchanged there.
Only where Windows does not have it does the stand-in take over:

- **Emulated** (`xp_sync.c`): the SRW locks, condition variables, `InitOnce` and FLS, written over XP's
  events and critical sections (a hashed parking lot; FLS destructors run from a TLS callback the
  object installs). These are the pieces that could silently corrupt, so they are stress-tested
  ([§6](#6-what-has-been-seen-running)).
- **Emulated** (`xp_misc.c`): `GetFileInformationByHandleEx`/`SetFileInformationByHandle` and the
  directory enumeration through XP's `NtQueryInformationFile` / `NtQueryDirectoryFile` /
  `NtSetInformationFile`; `GetFinalPathNameByHandleW` through the file's name and the drive whose
  volume serial matches; `CompareStringOrdinal`; `K32EnumProcessModules` = XP's `psapi` one;
  the locale from `GetLocaleInfoW`; `SetThreadErrorMode` = `SetErrorMode`; `CreateWaitableTimerExW` =
  `CreateWaitableTimerW`; the time zone from `GetTimeZoneInformation`; `RegGetValueW` and the
  transacted registry opens (the transaction ignored — XP has no Kernel Transaction Manager, and a
  single key's write needs none) and `RegDeleteTreeW` (recursive delete).
- **Answered "not available"** the way each function reports failure: `CreateSymbolicLinkW` (XP has no
  symlinks), `ReOpenFile`, the ProcThreadAttributeList trio (`std` uses them only for a caller's raw
  attributes, which RapidR never sets), `WSAPoll`/`WSASendMsg`. `SetThreadStackGuarantee` answers TRUE
  (reserving extra stack is not essential; `std` only debug-asserts it).
- **An empty DirectWrite** (`xp_dwrite.c`): Windows' own `DWriteCreateFactory` first; on plain XP a
  static factory whose system font collection has no families (every other method answers
  E_NOTIMPL, each popping its own stdcall arguments). `fontique` then finds no system fonts instead of
  panicking, and RapidR draws with its bundled fonts.
- **Graphical, answered "not available" on plain XP** (`xp_misc.c`): UI Automation, DXGI,
  DWM, touch, display config, DPI, window subclassing, the shell item/stock-icon calls, the
  PROPVARIANT/VARIANT conversions, `NCryptFreeObject`. A console program never calls these; a GUI
  program on plain XP stops with a clear failure instead of the loader refusing the whole `.exe`. On XP
  with One-Core-API many of these are the real ones.

XP-only code paths are compiled only into the XP flavour; nothing here weakens security or correctness
on any other Windows version.

### A trap in the build, and its fix

Cargo keeps one linked artifact per set of rustc arguments and relinks only when the arguments (or
inputs it tracks) change. The stand-in objects reach the link through the linker driver's
environment (`XP_SHIM_OBJS`), which cargo does not see: a rebuild after changing only a stand-in, with
the same arguments, handed back the previous link (seen 2026-11-05: the old DirectWrite answer
survived a rebuild, and `llvm-objdump` showed the old function in the new `.exe`). `build.sh` now
names an extra (empty) library folder after a hash of the stand-ins, the linker driver and the Windows
7 stand-in source, so any change makes cargo link again; the folder changes nothing in the program.
The same mechanism is why the Windows 7 programs come back byte-identical: their own artifact is kept
apart and handed back unchanged.

## 5. The go/no-go and the numbers

**GO for "console program runs on plain XP"** (the brief's threshold: about a day). The map is
bounded (85 functions, all understood), the mechanism is the proven Windows 7 one, and the dangerous
emulations pass a stress test against Windows' own semantics. No third-party code, no crate, no
toolchain fork (so no lawyer item): the stand-ins are RapidR's own, clean-room from Microsoft Learn.

Order, as the brief sets it, and where each stage got to (all on plain XP SP3, no One-Core-API, [§6](#6-what-has-been-seen-running)):

1. **Console: done.** `rapidr.exe` loads (header 5.1, zero XP-missing imports —
   [§7](#7-what-this-lane-cannot-affect-the-isolation-evidence)); `rapidr version` and a console
   program run.
2. **GUI: done.** A form opens and its events work. It needed one more stand-in: `fontique` unwraps
   DirectWrite's answer, so plain XP (no `dwrite.dll`) panicked; `xp_dwrite.c` hands it an *empty*
   DirectWrite factory (no system fonts), and RapidR draws with its bundled Liberation fonts.
   **RapidR Studio: done** — it starts, shows the designer, F5 runs the program (the os error 123
   case), and its Build button builds.
3. **Build (interpreted): done.** From Studio and from `rapidr build --interp`; the built program
   (the runner stub with the bytecode) runs on XP on its own.

(2026-11-06: the file dialogs, the check mark, Ctrl shortcuts and Build on 32-bit Windows below are
fixed now, and folder listings, which failed too: [§9](#9-preview-6-re-check-2026-11-06).)

Known limits on plain XP, each a "not available", not a crash: no DirectWrite system fonts (a symbol
the bundled fonts lack shows as an empty box: the check mark in Studio's "Built ..." line —
**needs a symbol fallback font among RapidR's bundled fonts**, a font-stack task, not this lane's);
no Vista file dialogs (`rfd`: Studio's File > Open / Save As dialogs answer nothing on plain XP —
opening by command line, the Project pane and Build work; an XP-only dialog stand-in over
`GetOpenFileNameW` is possible later); no UI Automation (no screen reader); CPU drawing only; an SSE2
CPU is required (Pentium 4 / Athlon 64 or newer, as the Rust target is built); TLS 1.0 and no SNI (AI
providers and most downloads will not connect). Studio shortcuts with Ctrl/Alt typed by
`WshShell.SendKeys` did not reach Studio in the VM (F5 and plain keys did; the toolbar's Build button
and dialogs worked by mouse) — to look at with a real keyboard on XP.

**Not this lane's:** on 32-bit Windows, Studio's Build still asks "Native builds need Rust" (with
*Build Interpreted Instead*); Robert's decision of 2026-11-04 (a single "Build" that makes the
interpreted app there) belongs to the **everybody-builds** lane.

## 6. What has been seen running

On Robert's Windows 7 VM in UTM (i686, the real Windows loader), through the test helper
(`tools/win7/vm.sh`). Each test runs twice: with Windows' own functions (which validates the test
against the real semantics) and with the XP emulation forced on (`ours`), so the emulation is checked
where the real functions also exist.

- **`tools/winxp/probe/pipeprobe.exe`** (the os error 123 fix): `std`'s pipe pair works; the XP named
  fallback works; a real `cmd /c echo` child's output comes back through the fallback pipe. Normal and
  forced-named runs both pass.
- **`tools/winxp/shim/tests/test_sync.exe`** (the dangerous emulations — SRW locks, condition
  variables, `InitOnce`, FLS): 13 tests — 800,000 exclusive increments across 8 threads with no lost
  update; try-lock; readers/writers with no torn read; a bounded queue (100,000 items, sums equal, 1 ms
  timed waits); a timeout at 200 ms reporting error 1460; wake-all waking 12 threads; shared-mode
  condition waits; `InitOnce` (one initializer, all 16 threads see the value; a failed init begun
  again); FLS destructors (7 of 8 threads, right values, right count); the last-error value kept; no
  recursion; a 6-thread random mix; and 8 seconds of timed waits racing wakes (3.0 M waits, 27 M
  wakes). **All pass with Windows' own functions and all pass with the emulation forced on**, including
  four concurrent instances (3 emulated + 1 native) under CPU contention.
- **`tools/winxp/shim/tests/test_misc.exe`** (the file, string, locale and registry emulations):
  `CompareStringOrdinal`; `GetFileInformationByHandleEx` (standard, basic, directory enumeration finds
  the temp file); `SetFileInformationByHandle` (truncate); `GetFinalPathNameByHandleW` (a drive-letter
  path); `GetUserDefaultLocaleName` ("en-US"); a registry round-trip through the transacted create, a
  value set and read, and `RegDeleteTreeW`. **All pass, Windows' own and emulation, the same results.**

### On plain Windows XP (Robert's UTM VM, 2026-11-05/06)

"Microsoft Windows XP [Version 5.1.2600]", Service Pack 3 (registry `CSDVersion`), no One-Core-API, one
emulated i386 CPU, host-only network (no internet). Driven through the XP test helper
(`tools/winxp/vm.sh`); the pictures are taken in the VM by `xpshot.exe` and pulled back.

- **`pipeprobe.exe`**: `std`'s own call, `NtCreateNamedPipeFile` with an empty name, answers
  **`0xC0000033` (STATUS_OBJECT_NAME_INVALID)** — the os error 123, measured on XP itself; the
  `xp_pipes.c` way makes the pipe and a real child's output comes back through it.
- **`test_sync.exe`** (here the emulation is the only implementation): all 14 pass, including 8 s of
  timed waits racing wakes (12.0 M waits, 50.0 M wakes) on XP's single CPU.
- **`test_misc.exe`**, **`test_dwrite.exe`**: all pass (no `dwrite.dll` on this XP: the empty factory
  answers; the final path has its drive letter).
- **RapidR itself** (the plain-XP flavour, header 5.1):
  - console: `rapidr.exe version` → `RapidR 2.122.1`; `rapidr.exe run hello.rr` → its output, exit 0
    (capture `xp-console.png`);
  - a form: `hello_form.rr` opens ("Hello, RapidR", XP's own title bar, RapidR's widgets), and Enter
    on its default button answers "Hello, World! (1)" (`xp-form-3.png`, `xp-form-greet.png`). Before
    the DirectWrite stand-in it stopped with fontique's panic (`xp-form-1.png`);
  - **RapidR Studio** (`rapidr.exe ide <example>`, the Windows 7 alpha's Studio files): starts in under
    two minutes on the emulated CPU, the designer shows the form (`xp-studio-1.png`); **F5** runs the
    program as Studio's child process — the reported failing case — "Running hello_form.rr", the program's
    window in front (`xp-studio-f5.png`); closing it: "[hello_form.rr ended, exit code 0]";
  - **Build** from Studio's toolbar → "Native builds need Rust" → *Build Interpreted Instead* →
    "Built interpreted program: C:\Documents and Settings\...\build\hello_form.exe (1554 bytes of
    bytecode) ... Build succeeded!" (`xp-studio-build-2.png`, `xp-studio-build-3.png`);
  - **the built `hello_form.exe`**, started on its own: its window, a name typed in, Greet →
    "Hello, Windows XP! (1)" (`xp-built-exe.png`, `xp-built-exe-greet.png`).

  The captures are in `/Volumes/RapidRBuild/wt-winxp/captures/` (outside git: pictures of the VM).

The build matrix's **`winxp-real`** row ([docs/build-matrix.md](build-matrix.md)) repeats the console,
form, Build (interpreted), built-program and Studio checks on the XP test zip
(`tools/winxp/zip.sh`) through the helper, so the release gate covers XP from now on.

## 7. What this lane cannot affect (the isolation evidence)

The XP flavour builds the Windows 7 libraries with the Windows 7 build's own flags and relinks only the
three programs with the XP object. To prove the normal builds are untouched, `tools/winxp/build.sh`
builds the Windows 7 `rapidr.exe` before and after and the hashes match:

```
baseline Windows 7 rapidr.exe            bbb752c4af6fa923e2edc529d1fb191ac61af436cf198881325c980fd07d5153
after the XP 5.1 build (and restore)     bbb752c4af6fa923e2edc529d1fb191ac61af436cf198881325c980fd07d5153   (identical)
after the XP 6.0 build (and restore)     bbb752c4af6fa923e2edc529d1fb191ac61af436cf198881325c980fd07d5153   (identical)

after the rebase on development (67289cdb, whose crates changed: a new baseline)
Windows 7 rapidr.exe, after each of the six later XP builds   134b5d3423d86bb4c712982d5556c30509d958e33273bf33e7dabba84a6c842b   (the same every time)
```

The XP flavour's own `rapidr.exe` differs, as intended (it carries the XP object). Its import table,
compared with the Windows 7 one (`tools/winxp/imports.sh <win7.exe> <xp.exe>`):

- **gone**: all 85 functions plain XP lacks — and with them seven whole DLLs XP does not have
  (`propsys`, `uiautomationcore`, `dwrite`, `dxgi`, `dwmapi`, `ktmw32`, `ncrypt`); the object answers
  for them now. 637 imports from 27 DLLs became **555 imports from 19 DLLs**;
- **added**: five `advapi32` functions the pipe stand-in's owner-only security descriptor uses
  (`InitializeSecurityDescriptor`, `SetSecurityDescriptorDacl`, `InitializeAcl`, `AddAccessAllowedAce`,
  `GetLengthSid`), all present on XP;
- **result** (`tools/winxp/imports.sh`): **0 functions missing on plain Windows XP SP3**, for both the
  5.1 (plain XP) and the 6.0 (XP + One-Core-API) header.

The source tree, the `Cargo.toml`s, the `RUSTFLAGS` and the build scripts of the normal build are
unchanged — `git status` shows only new files under `tools/winxp/` and this document. The macOS, Linux,
web and Windows 10/11 builds do not read `tools/winxp/` at all, so their inputs, and therefore their
outputs, are untouched. The build matrix's Windows and Windows 7 rows are to be re-run green before this
lane is reported done.

## 8. Legal

No One-Core-API code, no Microsoft DLLs or headers, no Wine/ReactOS/mingw-w64 code: every stand-in is
RapidR's own, written from Microsoft Learn's public documentation of each function and from the PE/COFF
import naming (`tools/winxp/clean-room.md`). No new crate, no toolchain fork (rust9x was considered and
rejected: a lawyer check, it lags Rust versions, and it is MSVC-only). So there is nothing new to
license. One-Core-API stays the user's own install; RapidR never ships or copies it.

## 9. Preview 6 re-check, 2026-11-06

Lane WINXP-RECHECK, on development `dad9346f` (RFormEx, context menus 2b, zh-Hans, pt-BR, JIT-GAPS,
RMIDIPort and async DLL memory merged). The same plain Windows XP SP3 VM (one emulated i386 CPU in UTM
on an Apple M4 Max, host-only network, no One-Core-API). Everything was run **in the front of the XP
desktop** for Robert to watch (helper v4's `vm.sh show`, each program closed by its own process
number), with a picture of each step. The pictures and logs are in
`/Volumes/RapidRBuild/lane-evidence/winxp-recheck/` (outside git).

### What was found and fixed

Four gaps on plain XP were closed in the XP flavour only (`tools/winxp/`, compiled into nothing else),
and three in shared code with the lead's go (each its own commit, correct on every system):

| Gap on plain XP | Cause (measured on XP) | Fix | Where |
|---|---|---|---|
| File > Open / Save As, a program's ROpenDialog / RSaveDialog and folder pickers answered "cancelled" | rfd asks COM for Vista's `CLSID_FileOpenDialog` / `CLSID_FileSaveDialog`; XP has no such class | an Open / Save / folder dialog of ours with the same COM interface, over XP's `GetOpenFileNameW` / `GetSaveFileNameW` / `SHBrowseForFolderW`; Windows' own first, always | XP flavour: `shim/xp_filedlg.c` |
| No folder could be listed (`std::fs::read_dir`): Studio's language list had only English | `FindFirstFileExW` with `FindExInfoBasic` (Windows 7) answers **error 87** on XP | the same call at the standard level after that refusal | XP flavour: `shim/xp_misc.c` |
| Ctrl shortcuts did nothing (Ctrl+S, Ctrl+O, Ctrl+Shift+B; a program's Ctrl+A); typed text, F keys and Alt+F4 worked | winit builds its keyboard-layout table with `MapVirtualKeyExW` kind 4 (Vista), which answers **0** for every key on XP | where kind 4 answers 0: kind 0's scan code, with 0xE0 for the extended keys | XP flavour: `shim/xp_misc.c` |
| ✓ ✗ ⚠ ✋ drew as empty boxes (Studio's "✓ Built ..." line, RAI's steps) | no DirectWrite on XP, so no system fonts; RapidR's own faces have no such symbols | **RapidR Symbols Subset** (58 symbols cut from the repository's Noto Sans Symbols 2, OFL): the last family **only** where the computer has no system fonts at all | shared (lead's option A): `rapidr_ui_kernel::text::symbols_last_resort` |
| Studio in Chinese: boxes everywhere and no word about it | plain XP has no Chinese font (East Asian support not installed); the note was Linux-only, and read `Application.OS`, which is not a member (always "") | the note on any desktop with no Chinese font, the system's own way to add one (Windows XP: Regional and Language Options > Install files for East Asian languages, from the Windows CD; newer Windows: add Chinese (Simplified)); the system from `RSystemInfo.OS` | shared: `ide/chrome.inc` |
| Build asked "Native builds need Rust" | Robert's decision of 2026-11-04 had not landed | on 32-bit Windows one **Build (Interpreted App)**, no question (`Proj.NativeBuilds`); 64-bit systems unchanged (unit test) | shared: `rapidr-studio`, `ide/build.inc`, `ide/commands.inc` |
| `RSystemInfo.OSVersion` said "10 (2600)" on XP, "10 (7601)" on 7 | the version came from the registry's build number alone | `RtlGetVersion`: "5.1 (2600)" on XP, "6.1 (7601)" on 7, 10 and 11 as before; checked on XP, Windows 7 and Windows 11 | shared: `rapidr-value/src/sysinfo.rs` |

### The nine items (and the tenth)

| # | Item | Result on plain XP | Pictures and logs |
|---|---|---|---|
| 1 | Import tables of `rapidr.exe` and both runner stubs | **0 functions plain XP lacks** (565 / 557 / 557 imports, 19 DLLs). New since the first XP build: `shell32!SHFileOperationW` (MOVETOTRASH), `user32!SetLayeredWindowAttributes` (RFormEx), `winmm!midiIn*` (8, RMIDIPort), `ntdll!RtlGetVersion` (OSVersion): all on XP. `CoCreateInstance` left the import table (the dialog stand-in answers it). Two run-time refusals the import table cannot show were found and fixed (`FindFirstFileExW`, `MapVirtualKeyExW`; `xp-missing.txt` lists them) | `tools/winxp/imports.sh` |
| 2 | Languages | **Works.** English, Español (switched live from Settings), Português (Brasil) with its accents, 中文: boxes (no Chinese font on this XP) and Studio's note in English and Chinese saying what to install from the Windows CD. Settings lists all five once folders can be listed | `xp-studio-en-2`, `xp-studio-es`, `xp-studio-pt-BR`, `xp-studio-pt-BR-languages`, `xp-studio-zh-2` (before the fixes: `xp-studio-settings-language`, `xp-studio-zh`) |
| 3 | RMIDIPort (`examples/media/midi_monitor.rr`) | **Works, refused in plain words where it must.** winmm lists no MIDI device in this VM (it has no sound card): both lists empty, "No input open; no output open"; Send with no output: "No MIDI output is open: call OpenOutput first"; no crash. Scripted devices (`RAPIDR_TEST_MIDIPORT`) not used: a release build's test hooks need the test-mode token, which stays off the VM | `xp-midi-monitor-1`, `-outputs`, `-send` |
| 4 | Async DLL memory | **Works.** `dll_byref_assigned_windows.bas` (GetComputerNameA, BYREF): "r =1 / gives1", RC.EXE's answer; `examples/windows/wndproc_postmessage.rr` (a WndProc callback): "Posted 2, arrived 2" | `xp-dll-getcomputername.txt`, `xp-dll-wndproc-2` |
| 5 | RFormEx (`examples/gui/form_ex.rr`) | **Works.** On top, the hole (the desktop shows through), see-through (layered window) and the tray icon all work on XP | `xp-form-ex-1`, `-2` (hole), `-3` (see-through), `-tray` |
| 6 | Right-click menus | **Works.** Studio's Project tree, code editor and tabs by right click; Shift+F10 and the Menu key (after the keyboard fix); `examples/gui/context_menus.rr` by right click and Menu key | `xp-studio-rclick-project`, `-code`, `-tab`, `xp-studio-shift-f10`, `xp-studio-menu-key-2`, `xp-context-menus-rclick`, `-menukey` |
| 7 | The JIT and the speed baseline | **Refused in plain words, interprets.** `RAPIDR_JIT=on` / `always`: same answers, exit 0; with `--verbose`: "JIT: not on this kind of computer; the interpreter runs everything" (no JIT for 32-bit x86: the XP-JIT lane). Speed below | `xp-speed-baseline.txt`, `xp-speed-rounds-2-3.txt` |
| 8 | Studio: F5, Build, the built program, Help, Settings | **Works.** F5 runs `hello_form.rr` ("Hello, XP! (1)", Alt+F4 closes it, "[ended, exit code 0]"); Run > **Build (Interpreted App)** builds with no Rust question, "✓ Built hello_form.exe (interpreted) in 4 s" with its check mark; the built `hello_form.exe` runs on its own ("Hello, WINXP! (1)", Ctrl+A first); F1 opens Help; File > Open shows XP's own dialog and opens the file; Settings opens | `xp-studio-runmenu`, `xp-studio-build`, `xp-studio-f5`, `-f5-greet`, `-f5-closed`, `xp-built-exe-greet`, `xp-studio-help`, `xp-studio-filemenu`, `xp-studio-open-dialog`, `xp-studio-opened-file`, `xp-studio-settings-2` |
| 9 | Big numbers, UINT32, `HEX$(x, digits)`, LOWORD ... (console) | **Works**, the same output as macOS, Windows 7 and Windows 11 | `xpnums-xp.txt` (`-macos`, `-win7`, `-win11`) |
| 10a | File dialogs | **Fixed** (XP flavour, above). `test_filedlg.exe`: 12 of 12 on XP (open, cancel, save, two files, a folder, another COM class still Windows' own); in Studio: File > Open | `xp-filedlg-test-2`, `xp-studio-open-dialog` |
| 10b | The check mark | **Fixed** (shared, above). The same program on XP before / after; on macOS before and after byte-identical | `xp-symbols-before`, `xp-symbols-after`, `mac-symbols-before`, `mac-symbols-after` |
| 10c | Build on 32-bit Windows | **Fixed** (shared, above) | `xp-studio-runmenu`, `xp-studio-build` |
| 10d | Ctrl / Alt shortcuts | **Fixed** (XP flavour, above): the keys sent as a keyboard sends them (`probe/xpkeys.exe`: virtual key and scan code) reach Studio: Ctrl+Shift+B builds | `xp-studio-ctrl-shift-b` (before), `xp-studio-keys-ctrl-shift-b` (after) |

### Speed on XP (the XP-JIT lane's baseline)

Emulated, not real hardware: one i386 CPU emulated by UTM (QEMU TCG) on an Apple M4 Max; the same VM,
the same programs, three rounds each, the first XP build (`2.122.1`, `edcd52d9`) against this one.
`loop10m` is the whole run (start to end, as `%TIME%` sees it); the others print their own time.

| Program | First XP build (rounds 1 / 2 / 3) | This build | |
|---|---|---|---|
| `tools/bench/vm/loop10m.rr` (10 M additions, whole run) | 3.70 / 3.51 / 3.33 s | 3.60 / 3.51 / 3.65 s | the same |
| `tools/bench/vm/fcalls.rr` (2 M calls) | 2890 / 2875 / 2875 ms | 3250 / 2953 / 2937 ms | about 2 % slower after the first round |
| `tools/bench/vm/dsp_voices.rr` (the synth: 32 voices) | 5187 / 4562 / 4562 ms | 4937 / 4578 / 4531 ms | the same |
| `examples/basics/hot_cpu.rr` (an 8-bit CPU, 1 M steps) | 12703 / 12546 / 12750 ms | 7000 / 6421 / 6531 ms | **1.9x faster** (the interpreter's work since) |

Every answer is the same in every run. The JIT is not built for 32-bit x86 (Cranelift has no x86-32
back end): `RAPIDR_JIT=on` or `always` interprets, and `--verbose` says so.

### Not checked, and why

- **Chinese with a Chinese font on XP**: plain XP has none and nothing is installed in the VM (East
  Asian support needs the Windows CD). XP's DirectWrite stand-in exposes no system fonts at all, so
  even an XP with SimSun installed draws boxes: making the XP flavour list XP's own fonts is a next step
  (the note then stops showing there by itself).
- **A real MIDI device** (none in the VM) and **scripted MIDI devices** (test-mode token not taken
  into the VM).
- **Files dropped** on RFormEx's DragZone (no drag source driven on XP).
- **Studio's Save As dialog** by hand (the dialog code path is the Open one, checked by `test_filedlg`
  Save cases; Studio's own Save As not clicked).
- **Screen reader** (XP has no UI Automation; MSAA would be needed) and **AI providers** (XP speaks only
  TLS 1.0 without SNI): report-only, below.

### Report-only: what each would take

- **A screen reader on XP**: AccessKit speaks UI Automation, which XP does not have. XP's screen readers
  read MSAA (`IAccessible` through `WM_GETOBJECT`): an MSAA bridge over the kernel's accessibility tree
  (name, role, state, value, children, focus and the events `NotifyWinEvent` sends) in the XP flavour
  or in the kernel. A few days, tested with a screen reader that still runs on XP.
- **AI providers on XP**: XP's Schannel speaks TLS 1.0 without SNI; every provider requires TLS 1.2
  and SNI. RapidR would need its own TLS for XP (`rustls` with a crypto provider that builds for i686
  and runs on XP), a lawyer check, and certificate roots of its own (XP's are out of date). A local
  model over `http://127.0.0.1` needs none of that, and works today wherever such a server runs.

### The build matrix

`tools/release/build-matrix.sh 2.122.1 --only winxp-real,win7real --shots`, with this branch's zips
(`tools/win7/alpha.sh`, then `tools/winxp/zip.sh`): **all green**. Windows XP SP3 (real, UTM VM): run,
Build (interp), the built programs and Studio ok, 1:45; Windows 7 (real, UTM VM): the same, 8:37.
Native builds n/a on both (no Rust for 32-bit Windows). Logs, the table and the pictures:
`lane-evidence/winxp-recheck/matrix/`.

### The isolation, again

The XP flavour's commits change only `tools/winxp/` (and this document, the manual). The Windows 7
programs were built twice at the same commit (pinned with `RAPIDR_COMMIT=isolation-check`:
`crates/rapidr-crash/build.rs` writes the commit into every program, so a new commit alone changes the
bytes): once with `tools/winxp/` in the tree, once with it moved out of the tree altogether. The same
bytes (`lane-evidence/winxp-recheck/isolation.txt`):

```
rapidr.exe              60fafd53da5081ea684abf8b4be2398e10daa03607224e1d2d09051a577c4cfa   (both)
rapidrintr-runner.exe   c3cff1bb70737e79fe7fe807b04116b4cb0c1ed18f8c16a5927beb2657b22b73   (both)
rapidrintr-runnerw.exe  58c459738de86cc2e89f420dc9ec18cfa6bd1318214e0556ae4a72f9dc1354e6   (both)
```

The macOS, Linux and web builds do not read `tools/winxp/` at all (only `tools/winxp/` itself,
`tools/release/winxp-vm.sh` and the build matrix's XP row name it). The
shared commits change, on every system: Studio's Build menu and Project Options on 32-bit Windows only
(64-bit unchanged); the Chinese-font note's condition and texts; `RSystemInfo.OSVersion` before
Windows 10; the font list only where a computer has no system fonts (byte-identical on macOS); a
desktop program's `THIRD-PARTY-NOTICES.txt` gains the symbols subset's licence; every desktop binary
grows by the subset's 9.7 KB.

## 10. Sound on XP, 2026-11-06 (lane XP-SOUND)

The XP VM got a sound card (an emulated Intel 82801AA AC'97, driver `ac97intc.sys` 5.1.2535.0, which
XP's winmm calls "Intel(r) Integrated Audio": one wave-out and one wave-in device, one MIDI out,
"Microsoft GS Wavetable SW Synth", no MIDI in) and a Core 2 Duo P9xxx (Penryn class) with 2 cores and
1023 MB. `RSystemInfo` on that XP: OS "Windows", OSVersion "5.1 (2600)", CPU "Intel Core 2 Duo P9xxx
(Penryn Class Core 2)", Cores 2, class "small". Pictures and logs:
`/Volumes/RapidRBuild/lane-evidence/xp-sound/` (outside git).

| What | On XP before the guard | Why |
|---|---|---|
| RMIDIPort (`midi_monitor.rr`: list, Test note, program change, notes), a console scale, drums | **Works**, and was heard | winmm's `midiOut`, through `midir` |
| RMIDI (`midi.rr`, `midi_compose.rr`) | **Works**, was heard (it plays on the GS Wavetable synthesizer) | the same winmm output |
| `RAudioOut` (`beeper.rr`, `tone_generator.rr`, `chiptune.rr`, `frame_audio.rr`) | **the program stopped** (exit 101, a crash report): "called `Result::unwrap()` on an `Err` value: HRESULT(0x80040154) Class not registered" | cpal 0.15's only Windows backend is WASAPI: it builds `IMMDeviceEnumerator` (mmdevapi, Windows Vista and newer) with an `unwrap` (`host/wasapi/device.rs:851`); XP has no such class |
| `PLAYWAV`, RWave's Play and Record | the program stopped (the same panic, on the main thread) | the same enumerator |
| `BEEP`, `SOUND`, `PLAYSOUND` | silent; a thread-panic crash report each, the program went on | the same, in their own thread |
| 3 rounds of the speed programs on this CPU | see the speed table below | |

The fix (shared, correct on every system; `crates/rapidr-runtime-core/src/sound.rs`): every call into
the sound backend goes through one guarded place. A panic inside it is caught (`catch_unwind` around
that call only), neither printed nor reported (`rapidr_crash::quiet_panics`), remembered (the backend
is not asked again) and told in plain words: "No sound output is available on this computer", and on
Windows before Vista "Sound output needs Windows Vista or newer for now". `RAudioOut.Error` says it
and the stream plays silently, in time; `PLAYWAV` says it on its error line; the program carries on.
Real sound on XP (winmm's wave output as an XP-flavour backend) is a later step.

After the guard, on the same XP (the XP flavour built from this branch, header 5.1, 562 imports, 0 missing):

| What | Result on XP |
|---|---|
| `RAudioOut` in a console program | creates and Starts (`Playing` 1), `Error` = "Sound output needs Windows Vista or newer for now: the stream plays silently (its timing goes on)", 3 s of writes in time (`Underruns` 0), exit 0, no crash report |
| `beeper.rr`, `tone_generator.rr`, `chiptune.rr`, `frame_audio.rr` | open and run (before: exit 101 at once); `tone_generator.rr`'s oscilloscope draws and its line reads "... Sound output needs Windows Vista or newer for now: the stream plays silently" (it counts many gaps there: the emulated XP is slower than the 5 ms the example's feed asks for; a real wave output will need its own look) |
| `PLAYWAV` | the program goes on; its error line says "[rapidr] PLAYWAV: Sound output needs Windows Vista or newer for now" |
| `BEEP`, `SOUND`, `PLAYSOUND` | silent, no crash, no crash report |
| `wave.rr` (RWave Chime, Record) | the window stays: "Chime: 1200 ms, 16-bit, 11025 Hz", Record runs to its end; no crash report |
| macOS, the real sound card | `RAudioOut`, `BEEP`, `SOUND`, `PLAYWAV`, `PLAYSOUND` play as before (heard) |

Tests: `sound::tests::a_panic_in_the_backend_is_an_error_not_a_crash` (the enumeration panics the way cpal's does:
the answer is the plain reason and the backend is not asked again), `an_error_of_the_backend_is_told_and_asked_again`,
`rodio_with_no_device_says_no_sound_output`, `rapidr-crash`'s `quiet_panics_nest_and_leave_even_after_a_caught_panic`;
`tests/examples_run.mjs --runtimes run media/ recipes/frame_audio`: 17 of 17 on macOS; the web runtime and the
build without the audio feature still compile.

### Speed on this CPU (Core 2 Duo P9xxx, 2 cores; the XP-JIT lane's baseline)

Three rounds, this XP build, the same programs as section 9 (loop10m whole run; the others print their
own time). The Mac was busy with other lanes (load average about 38) while these ran: the spread is
partly that.

| Program | Round 1 / 2 / 3 | Section 9's CPU (one i386 vCPU) |
|---|---|---|
| `tools/bench/vm/loop10m.rr` (10 M additions) | 3.93 / 5.06 / 3.69 s | 3.60 / 3.51 / 3.65 s |
| `tools/bench/vm/fcalls.rr` (2 M calls) | 3500 / 3687 / 3125 ms | 3250 / 2953 / 2937 ms |
| `tools/bench/vm/dsp_voices.rr` (32 voices) | 6062 / 5859 / 4687 ms | 4937 / 4578 / 4531 ms |
| `examples/basics/hot_cpu.rr` (1 M steps) | 9515 / 7406 / 8171 ms | 7000 / 6421 / 6531 ms |

Every answer is the same as before; `RAPIDR_JIT=always --verbose` still says "JIT: not on this kind of
computer; the interpreter runs everything".


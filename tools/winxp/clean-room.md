# Windows XP stand-ins (clean-room record)

Lane WINXP, 2026-11-05. The companion to `tools/win7/clean-room.md`, for the Windows XP flavour
(`tools/winxp/build.sh`, `docs/winxp-plan.md`). Robert's rule (memory `legal-zero-risk`): no code that
is not clearly ours. This records what was written, from what, and what was deliberately **not** read.

## What was written

All of `tools/winxp/shim/` and `tools/winxp/probe/`, by this lane's agent, in C:

- `xp_answer.h` — the `XP_ANSWER(ours, Func, bytes)` macro: for a Windows function `Func`, it defines
  the three names a caller in RapidR's programs can use for it — the import thunk `_Func@bytes`
  (Rust's `std`), the decorated import slot `__imp__Func@bytes` (C `dllimport`), and the undecorated
  slot `__imp_Func` (the `windows`/`windows-sys` crates built with `--cfg windows_raw_dylib`) — each
  aimed at a function of ours.
- `xp_common.h` — `xp_real(dll, name, &cache)`: Windows' own function if this Windows has it (loaded
  from Windows' own folder), looked up once; the last-error value kept.
- `xp_pipes.c` — `NtCreateNamedPipeFile` / `NtOpenFile`: the real ones first, and on XP's refusal of an
  unnamed pipe the same pipe made with a random name and an owner-only security descriptor (the
  os error 123 fix).
- `xp_sync.c` — the slim reader/writer locks, condition variables, `InitOnce`, FLS and `IsThreadAFiber`
  (real first, else emulated over XP's events and critical sections; a hashed parking lot; FLS
  destructors run from a TLS callback this object installs).
- `xp_misc.c` — the rest (file information, final path, string compare, locale, registry, time zone,
  `K32EnumProcessModules`, and the graphical functions a console program never calls): real first,
  else emulated the documented way or answered "not available".
- `xp_dwrite.c` — `DWriteCreateFactory`: Windows' own first, else an empty DirectWrite factory (two
  static COM objects: the factory and a font collection with no families). The method order and the
  interface IDs are those the `windows` crate declares (windows-0.62.2, MIT/Apache-2.0, the crate
  fontique itself uses), the methods' meaning from Microsoft Learn's IDWriteFactory and
  IDWriteFontCollection pages. No DirectWrite implementation of anyone else was read.
- `xp_filedlg.c` (lane WINXP-RECHECK, 2026-11-06) — `CoCreateInstance` and `SHCreateItemFromParsingName`:
  Windows' own first, always; only when Windows cannot make the Vista "Common Item Dialog"
  (`CLSID_FileOpenDialog` / `CLSID_FileSaveDialog`: plain XP) a dialog object of ours with the same COM
  interface (IFileOpenDialog / IFileSaveDialog), shown with XP's own `GetOpenFileNameW` /
  `GetSaveFileNameW` and, for folders, `SHBrowseForFolderW`; its results are IShellItem /
  IShellItemArray objects of ours holding the paths. Written from Microsoft Learn's pages for
  IModalWindow, IFileDialog, IFileOpenDialog, IFileSaveDialog, IShellItem, IShellItemArray,
  FILEOPENDIALOGOPTIONS, SIGDN, COMDLG_FILTERSPEC, OPENFILENAMEW, GetOpenFileNameW, GetSaveFileNameW,
  CommDlgExtendedError, BROWSEINFOW, SHBrowseForFolderW, BrowseCallbackProc, SHGetPathFromIDListW,
  CoCreateInstance and CoTaskMemAlloc; the method order and the interface / class IDs as the `windows`
  and `windows-sys` crates declare them (MIT/Apache-2.0); which methods rfd 0.17 calls from its
  `win_cid` backend (MIT; its `win_xp.rs.old` file was not opened). No other implementation of these
  dialogs was read. Its check: `shim/tests/test_filedlg.c`.
- `xp-link.sh` — the XP flavour's linker driver (puts the stand-in objects first in the link).
- `zip.sh` — the XP test zip (the Windows 7 zip with the plain-XP programs).
- `helper/START-HERE.vbs`, `helper/unzip.js`, `xphost.py`, `vm.sh` — the XP VM's test helper (VBScript /
  JScript of Windows XP's own Script Host, from Microsoft's documentation of WshShell, FileSystemObject,
  MSXML2.ServerXMLHTTP, ADODB.Stream, Shell.Application and WMI's Win32_Process).
- `probe/pipeprobe.c`, `probe/xpshot.c`, `probe/xpclick.c`, `shim/tests/test_sync.c`,
  `shim/tests/test_misc.c`, `shim/tests/test_dwrite.c`, `shim/tests/test_filedlg.c` — the checks run on the Windows 7 VM and on
  Windows XP; `tools/release/winxp-vm.sh` — the build matrix's XP row.

## Sources consulted

| Source | For |
|---|---|
| Microsoft Learn — the documentation pages of each Windows function the stand-ins replace or call | their signatures, parameters, return values and error reporting |
| Microsoft Learn — `NtCreateNamedPipeFile`, `NtOpenFile`, `NtCreateFile`, `NtQueryInformationFile`, `NtQueryDirectoryFile`, `NtSetInformationFile`, `OBJECT_ATTRIBUTES`, `UNICODE_STRING`, `IO_STATUS_BLOCK`, the `FILE_*_INFORMATION` structures and classes, NTSTATUS values, `RtlNtStatusToDosError` | the NT calls XP has that the emulations are built on |
| Microsoft Learn — `CreateEventW`, `WaitForSingleObject`, `InitializeCriticalSection`, the registry functions, `GetLocaleInfoW`, `SetErrorMode`, `GetTimeZoneInformation`, `EnumProcessModules`, `CreateWaitableTimerW`, `GetVolumeInformationW` | the XP building blocks |
| Microsoft's PE/COFF specification — the import-table naming (`__imp_`), stdcall `@N` decoration, the TLS directory (`_tls_used`) and TLS callbacks | the symbol names callers use and the thread-exit callback |
| Rust's `std` source (`library/std/src/sys/process/windows/child_pipe.rs`, `.../sync/...`, `.../thread_local/...`, `.../fs/windows.rs`), MIT/Apache-2.0 | which functions `std` calls and how, and the `NtCreateNamedPipeFile` signature as `std` declares it |
| RapidR's own `tools/win7/shim/win7_stand_ins.c`, `crates/rapidr-cli/src/win7.rs` and `tools/win7/clean-room.md` | the stand-in pattern this lane extends |
| `llvm-nm` / `llvm-readobj` on RapidR's own built libraries and programs | which functions are imported and by which library, and the exact symbol forms |

## Not read, not opened, not searched, not copied from

- **One-Core-API** (github.com/shorthorn-project/One-Core-API-Binaries) — its source or binaries, in any
  form. It stays the user's own install; RapidR never ships or copies it.
- **Wine** and **ReactOS** — their implementations of any of these functions (SRW locks, condition
  variables, the NT pipe or file calls, `RegDeleteTree`, anything).
- **mingw-w64** — its `delayimp.*` or any runtime source beyond the headers the compiler includes by
  itself (and this lane's C files include no system header: they declare the few functions they call).
- **Microsoft's** `delayhlp.cpp`, CRT or any SDK source.
- Any other implementation of these functions found anywhere.

The emulations were written from the documented behaviour of each function and the documented XP
building blocks, as original C. The parking-lot idea (a hashed table of wait queues) is a generally
described technique, implemented here from scratch.

## Checked after

- `tools/winxp/imports.sh <exe>`: the XP flavour's `rapidr.exe` has zero functions and zero DLLs that
  plain Windows XP SP3 lacks (`docs/winxp-plan.md` §7).
- The Windows 7 `rapidr.exe` built before and after the XP flavour's relink is byte-identical
  (same SHA-256): the stand-ins reach only the XP flavour.
- On the Windows 7 VM, the stand-ins were exercised against Windows' own functions and forced down
  their own emulated path, and agree (`docs/winxp-plan.md` §6).

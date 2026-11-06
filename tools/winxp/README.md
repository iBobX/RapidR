# RapidR on Windows XP (32-bit), built on the Mac

Lane WINXP, 2026-11-05. **Windows XP is unofficial.** The plan, the map, the go/no-go and what was seen
running on plain XP SP3 are in [`docs/winxp-plan.md`](../../docs/winxp-plan.md); the clean-room record is
[`clean-room.md`](clean-room.md). This builds on the Windows 7 flavour (`tools/win7/`): the same nightly
Rust, the same `i686-win7-windows-gnu` target and LLVM-MinGW (msvcrt), with an object of XP stand-ins
linked into the three programs and (for plain XP) the PE header set to 5.1. Nothing here is compiled into
any other build.

| File | What it does |
|---|---|
| `build.sh` | the XP flavour: `target/winxp-<header>/{rapidr,rapidrintr-runner,rapidrintr-runnerw}.exe`. `XP_HEADER=5.1` (plain XP) or `6.0` (default; XP + One-Core-API). It links the Windows 7 programs again afterwards so their files are unchanged (`WINXP_KEEP_WIN7=0` skips that). |
| `zip.sh <win7 zip>` | the XP **test** zip: the Windows 7 zip with its three programs replaced by the plain-XP ones (checked: header 5.1, nothing XP lacks), `README-WINXP-TEST.txt` added |
| `xp-link.sh` | the flavour's linker driver: puts the stand-in objects first in each link (`docs/winxp-plan.md` §4) |
| `shim/xp_answer.h`, `xp_common.h` | `XP_ANSWER` (defines the three import names a caller uses) and `xp_real` (Windows' own function if present) |
| `shim/xp_pipes.c` | `NtCreateNamedPipeFile` / `NtOpenFile`: the os error 123 fix (Studio's F5, Build, captured `SHELL`) |
| `shim/xp_sync.c` | SRW locks, condition variables, `InitOnce`, FLS, `IsThreadAFiber` (real first, else emulated) |
| `shim/xp_dwrite.c` | `DWriteCreateFactory`: real first, else an empty factory (no system fonts: RapidR's bundled fonts draw) |
| `shim/xp_filedlg.c` | `CoCreateInstance` / `SHCreateItemFromParsingName`: Windows' own first; where Windows has no Vista Open / Save dialog (plain XP), one of ours over XP's `GetOpenFileNameW` / `GetSaveFileNameW` / `SHBrowseForFolderW` (Studio's and programs' file and folder dialogs) |
| `shim/xp_misc.c` | file information, final path, string compare, locale, registry, time zone, `K32EnumProcessModules`, and the graphical functions a console program never calls |
| `shim/tests/` | `test_sync`, `test_misc`, `test_dwrite`, `test_filedlg` (+ their `build*.sh`): run on the Windows 7 VM against Windows' own functions and forced down the emulation (`ours`), and on Windows XP |
| `probe/` | `pipeprobe` (std's child pipes vs ours), `xpshot` (a screenshot: XP has none), `xpclick` (a mouse click), `build.sh` |
| `imports.sh`, `xp-missing.txt` | an executable's imports, and what plain XP SP3 lacks (Microsoft Learn's "minimum supported client") |
| `vm.sh`, `xphost.py`, `helper/` | drive Robert's XP VM in UTM (below) |

## Build and check

```
XP_HEADER=5.1 tools/winxp/build.sh                         # plain XP
tools/winxp/imports.sh target/winxp-5.1/rapidr.exe          # must say 0 missing on plain XP
tools/winxp/imports.sh <win7 rapidr.exe> target/winxp-5.1/rapidr.exe   # the difference from the win7 build
tools/winxp/zip.sh dist/<ver>/.../RapidR-<ver>-win7-x86-alpha<N>.zip  # the XP test zip
MATRIX_WINXP_ZIP=<that zip> tools/release/build-matrix.sh <ver> --only winxp-real
```

## The XP VM (UTM, "Windows XP": plain XP SP3, host-only network, no internet)

XP has no QEMU guest agent, so a helper of ours, `helper/START-HERE.vbs`, is started **once by hand** in
the VM (from `C:\xp`, or the CD `vm.sh iso` makes). It asks `xphost.py` on the Mac (192.168.128.1:8097,
the host-only bridge only) for work, runs as the user at the desktop (windows show there), installs
nothing and changes no setting; from version 2 it survives a failing job and `vm.sh update` replaces it
from the Mac (no more hand restarts).

```
tools/winxp/vm.sh host                     # start xphost.py (vm.sh stop-host stops it)
tools/winxp/vm.sh status                   # when the helper last asked (and its version: v=4)
tools/winxp/vm.sh exec 'ver'               # a command line, its output and "#exit N" (LIMIT=<s> stops it)
tools/winxp/vm.sh spawn 'C:\x\prog.exe'    # a program in a window of its own, not waited for
tools/winxp/vm.sh show 'C:\x\prog.exe'     # the same in the FRONT, for a person watching (v4): its process numbers
tools/winxp/vm.sh shot here.png             # a picture of the XP desktop
tools/winxp/vm.sh keys 'Title' '{F5}'      # keys typed into a window by its title (WshShell.SendKeys)
tools/winxp/vm.sh push local 'C:\x.exe'    # a file in;  pull 'C:\x.bmp' local: a file out
tools/winxp/vm.sh ps | kill <pid>          # processes; stop one by its number
tools/winxp/vm.sh update                   # the helper replaced by helper/START-HERE.vbs
```

Pictures: `C:\xp\xpshot.exe C:\xp\x.bmp`, pulled and converted with `sips`. Clicks: `C:\xp\xpclick.exe x y`.

We never bundle or copy One-Core-API; it stays the user's own install (`docs/winxp-plan.md` §8).

# RapidR on Windows XP (32-bit), built on the Mac

Lane WINXP, 2026-11-05. **Windows XP is unofficial.** The plan, the map and the go/no-go are in
[`docs/winxp-plan.md`](../../docs/winxp-plan.md); the clean-room record is
[`clean-room.md`](clean-room.md). This builds on the Windows 7 flavour (`tools/win7/`): the same
nightly Rust, the same `i686-win7-windows-gnu` target and LLVM-MinGW (msvcrt), with an extra object of
XP stand-ins linked into the three programs and (for plain XP) the PE header set to 5.1.

| File | What it does |
|---|---|
| `build.sh` | builds the XP flavour: `target/winxp/i686/{rapidr,rapidrintr-runner,rapidrintr-runnerw}.exe`. `XP_HEADER=6.0` (default; XP + One-Core-API) or `5.1` (plain XP). It also rebuilds the Windows 7 programs afterwards so their files are unchanged (`WINXP_KEEP_WIN7=0` skips that). |
| `xp-link.sh` | the flavour's linker driver: puts the stand-in objects first in each link (why: `docs/winxp-plan.md` §7, and the script's own header). |
| `shim/xp_answer.h`, `xp_common.h` | the `XP_ANSWER` macro (defines the import names) and `xp_real` (Windows' own function if present). |
| `shim/xp_pipes.c` | `NtCreateNamedPipeFile` / `NtOpenFile`: the os error 123 fix (Studio's F5, Build, captured `SHELL`). |
| `shim/xp_sync.c` | SRW locks, condition variables, `InitOnce`, FLS, `IsThreadAFiber` (real first, else emulated). |
| `shim/xp_misc.c` | file information, final path, string compare, locale, registry, time zone, `K32EnumProcessModules`, and the graphical functions a console program never calls. |
| `shim/tests/` | `test_sync.c` + `build.sh`, `test_misc.c` + `build_misc.sh`: stress and correctness tests, run on the Windows 7 VM against Windows' own functions and forced down the emulation. |
| `probe/` | `pipeprobe.c` (does this Windows make `std`'s child pipes; does the fallback work), `xpshot.c` (a screenshot, since XP has none), `build.sh`. |
| `imports.sh` | an executable's imports, and the ones plain XP SP3 lacks (`xp-missing.txt`). |
| `xp-missing.txt` | the functions/DLLs plain XP SP3 lacks (from Microsoft Learn's "minimum supported client"). |
| `vm.sh`, `xphost.py`, `helper/START-HERE.vbs` | drive Robert's XP VM in UTM (XP has no QEMU guest agent; the helper talks to the Mac on the host-only network only — the VM has no internet). |

## Build and check

```
tools/winxp/build.sh                          # XP + One-Core-API header (6.0)
XP_HEADER=5.1 tools/winxp/build.sh            # plain XP header (5.1)
tools/winxp/imports.sh target/winxp/i686/rapidr.exe        # must say 0 missing on plain XP
tools/winxp/imports.sh <win7 rapidr.exe> target/winxp/i686/rapidr.exe   # the difference from the win7 build
```

## The XP VM (when Robert has started the helper)

XP has no QEMU guest agent, so a small helper of ours (`helper/START-HERE.vbs`, started once by hand in
the VM) talks to `xphost.py` on the Mac over UTM's host-only network (192.168.128.1:8097) — never the
internet. The helper installs nothing and changes no setting.

```
tools/winxp/vm.sh iso           # the helper disc ($SHARE/RAPIDR-XP-HELPER.iso) to attach in UTM
tools/winxp/vm.sh host          # start xphost.py (host-only; Ctrl+C or vm.sh stop-host to stop)
tools/winxp/vm.sh status        # when the helper last asked for work
tools/winxp/vm.sh exec 'ver'    # a command line in the VM, its output
tools/winxp/vm.sh push local 'C:\x.exe'
tools/winxp/vm.sh pull 'C:\x.png' local
```

We never bundle or copy One-Core-API; it stays the user's own install (`docs/winxp-plan.md` §8).

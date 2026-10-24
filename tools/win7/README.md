# RapidR for Windows 7 / 8 / 8.1 (32 and 64 bit), built on the Mac

Lane WIN7-1, 2026-10-23. Background and the feasibility study: `docs/old-pcs-spike.md`.
These builds are **separate from the release** (stable Rust, Windows 10+): they use a
nightly Rust and the `*-win7-windows-gnu` targets, which run on Windows 7 and later.

| File | What it does |
|---|---|
| `build.sh [i686\|x86_64] [cargo args]` | builds `rapidr.exe` (default `-p rapidr-cli`) |
| `imports.py rapidr.exe [--against older.exe]` | lists the DLL functions the .exe needs at start, flags what Windows 7 lacks; `--against` lists what is new since an .exe known to start there. The Windows 7 .exe has no delay-loaded section any more |
| `stage.sh [i686\|x86_64]` | the test drop: stripped exe, Studio's files, tests, READ-ME, a `.zip`, and the disc image (ISO) |
| `serve.sh` | serves the drop to the VM over the **host-only** network only (HTTP, read-only) |
| `alpha.sh [N]` | the **alpha download**: `RapidR-<version>-win7-x86-alpha<N>.zip` + `.sha256` in `dist/<version>-win7-alpha<N>/` (`alpha/` holds its README-ALPHA.txt and launcher, `licenses/` the toolchain's licence texts; `THIRD-PARTY-NOTICES.txt` is made by `rapidr notices tools-windows` from any rapidr built for the Mac: `HOST_RAPIDR`, default `/Volumes/RapidRBuild/main/target/release/rapidr`) |
| `link_audit.sh <mingw-w64 source>` | relinks with a map and traces every toolchain object in the .exe to its source and licence (review: `docs/legal/review-2026-10-23-win7-alpha.md`) |
| `tests/` | `run-tests.bat` (console + window smoke tests), `run-studio.bat`, `form_auto.rr` |

## One-time setup (nothing system-wide, no sudo)

1. `rustup toolchain install nightly --profile minimal` and `rustup component add rust-src --toolchain nightly`.
2. The linker and C compiler: **LLVM-MinGW 20261020**, put in `/Volumes/RapidRBuild/tools/llvm-mingw-msvcrt`
   (`LLVM_MINGW` overrides). LLVM publishes its macOS build only with the UCRT C runtime, and
   Windows 7 has `msvcrt.dll` but not the Universal CRT. So the macOS build is combined with the
   msvcrt sysroot from the same release:
   - `llvm-mingw-20261020-ucrt-macos-universal.tar.xz` (124,415,944 bytes, sha256 `e9c13e4e…c17e`): the tools.
   - `llvm-mingw-20261020-msvcrt-x86_64.zip` (172,961,237 bytes, sha256 `52a3e3f5…8ff67`): its
     `i686-w64-mingw32/{bin,lib,share}` and `x86_64-w64-mingw32/{bin,lib,share}` replace the macOS
     build's, and its `include/_mingw.h` replaces `generic-w64-mingw32/include/_mingw.h` (the only
     header that differs). A test program compiled and linked this way imports `msvcrt.dll` and
     `KERNEL32.dll` only.
   - Licences: LLVM, clang, lld, compiler-rt, libunwind: Apache-2.0 with LLVM exception (`LICENSE.TXT`
     at the tree's root). The mingw-w64 headers and C runtime that get linked into the .exe are ZPL-2.1 /
     public domain / BSD-style (the archive does not carry their texts; upstream's `COPYING.MinGW-w64-runtime`).
     The archive also holds `gendef` and `widl` (GPL / LGPL, `share/*/COPYING.txt`): build tools we never run
     or link. Nothing from the tree ships in RapidR's downloads except what the linker copies into the .exe.

## Build, check, test

```
tools/win7/build.sh i686                      # -> target/i686-win7-windows-gnu/release/rapidr.exe
python3 tools/win7/imports.py target/i686-win7-windows-gnu/release/rapidr.exe
tools/win7/stage.sh i686                      # -> /Volumes/RapidRBuild/win7-share (+ the ISO)
tools/win7/serve.sh                           # with the VM running; Ctrl+C when done
```

## Windows 7 without Service Pack 1 (the test VM is build 7600, RTM)

By name, every function the .exe imports at start-up is in Windows 7 RTM (UI Automation 3.0,
DirectWrite, DXGI 1.1, the touch API, the `K32*` process functions, SChannel); none is SP1-only. That is
a reading of Microsoft's "minimum supported client", not a proof: the proof is starting it there.
What a PC without updates may still lack (to watch, not yet seen): TLS 1.2 through SChannel is there but
off by default (a program must ask for it), RSA-certificate servers get CBC suites only (no GCM), and
the root certificates are as old as the PC's last update: AI providers and downloads can fail to verify
on an offline, never-updated Windows 7. Nothing that needs the network is part of the smoke tests.

## What had to be done for Windows 7

- `tray.rs`: the 32-bit `NOTIFYICONDATAW` is a packed struct (the spike's 3-line fix).
- The `windows` / `windows-sys` crates use six functions Windows 7 lacks
  (`RoGetActivationFactory`, `RoOriginateErrorW`, `PathCchStripPrefix`, `CoIncrementMTAUsage`, and
  `CoCreateFreeThreadedMarshaler` / `CoTaskMemFree`, which Windows 7 has in `ole32.dll` but the crates take from
  `combase.dll`). `build.sh` builds with `--cfg windows_raw_dylib`, which makes every call go through an
  `__imp_<Function>` pointer; `crates/rapidr-cli/src/win7.rs` defines those six pointers itself, each aimed at a
  small function of RapidR's that finds the real function on first use (`GetProcAddress`) or answers "not
  available". Nothing is delay-loaded and no delay-load helper is linked (the one in mingw-w64 is said to derive from
  Microsoft's: `tools/win7/clean-room.md`). On Windows 8 and later the real functions are found and nothing changes.
  `rapidr.exe __win7_check` shows where each of the six was found.
- LLVM-MinGW has no `libgcc`: `build.sh` makes empty/alias `libgcc.a`, `libgcc_eh.a`, `libgcc_s.a` (compiler-rt
  and libunwind do the work) in the target folder.

## Automating the Windows 7 VM (UTM, no clicks)

The test VM is a UTM virtual machine named **Windows 7 x86** (`utmctl list`): i386 `pc`, 3 GB, 2 cores, an IDE disk
and an empty IDE CD drive, an Intel e1000 network card in **Host Only** mode (no internet), `virtio-vga-gl` display.
Windows 7 RTM, no updates. Keep it offline.

What works from the Mac (checked 2026-10-23):

| What | How |
|---|---|
| state | `utmctl status "Windows 7 x86"` (or `tools/win7/vm.sh status`) |
| its settings, read-only | `osascript -e 'tell application "UTM" to get configuration of virtual machine "Windows 7 x86"'` |
| files to the guest, 1: the host-only share | `tools/win7/serve.sh` (in a second terminal), then in the guest Internet Explorer or `powershell -Command "(New-Object Net.WebClient).DownloadFile('http://192.168.128.1:8099/<file>','C:\<file>')"` |
| files to the guest, 2: a disc | `tools/win7/alpha.sh` with `WIN7_ISO_DIR` makes `RAPIDR-ALPHA1.iso`; attach it with UTM's *Drives* button |

What needs the **QEMU guest agent** (`utmctl exec`, `utmctl file push/pull`, `utmctl ip-address`) is **not set up yet**:
`utmctl exec "Windows 7 x86" --cmd "cmd.exe /c ver"` answers *"The QEMU guest agent is not running or not installed on the guest."*
Installing it needs the guest's keyboard once, because nothing can type into the VM's display from the Mac
(UTM's AppleScript has no keystroke command; the display is a canvas that background automation cannot click).

### One-time install of the guest agent (about 3 minutes)

The files are virtio-win 0.1.173 (the last release with Windows 7 drivers; `/Volumes/RapidRBuild/tools/virtio-win-0.1.173-9/virtio-win-0.1.173.iso`,
unpacked pieces in `/Volumes/RapidRBuild/win7-share/guest/`): the *VirtIO serial* driver (`vioserial\w7\x86`) and `guest-agent\qemu-ga-i386.msi`.

1. Mac: `tools/win7/vm.sh stage` (puts `install-agent.ps1` on the share), then `tools/win7/serve.sh` (leave it running).
2. Windows 7 (VM running): Start, type `cmd`, press **Ctrl+Shift+Enter** (Administrator; answer **Yes**), and type
   ```
   powershell -NoProfile -ExecutionPolicy Bypass -Command "iex (New-Object Net.WebClient).DownloadString('http://192.168.128.1:8099/install-agent.ps1')"
   ```
   It downloads the four driver files and the MSI to `C:\agent`, runs `pnputil -i -a vioser.inf` (Windows asks *"Windows can't verify
   the publisher"*: choose **Install this driver software anyway**), installs the agent service and writes `C:\agent\install.log`.
3. If Device Manager shows "PCI Simple Communications Controller" without a driver afterwards, restart the VM once.
   If there is **no** such device at all, the VM has no virtio-serial port: UTM adds one for the guest agent on QEMU
   machines; if it did not, stop the VM and add it in UTM, *Edit, QEMU, Arguments*: `-device virtio-serial-pci`.
4. Mac: `tools/win7/vm.sh exec ver` must print `Microsoft Windows [Version 6.1.7600]`. Stop `serve.sh`.

### After that (written before the agent existed: untested, check each line the first time)

```sh
tools/win7/vm.sh push dist/2.121.0-win7-alpha1/RapidR-2.121.0-win7-x86-alpha1.zip 'C:\alpha.zip'    # CAP=300 for big files
tools/win7/vm.sh exec 'powershell -NoProfile -Command "$s=New-Object -ComObject Shell.Application; $s.NameSpace(\"C:\\\").CopyHere($s.NameSpace(\"C:\alpha.zip\").Items(),16)"'
tools/win7/vm.sh exec 'cd /d C:\RapidR-2.121.0-win7-x86-alpha1\tests && run-tests.bat < nul'          # the tests pause at the end: `pause` needs input
tools/win7/vm.sh pull 'C:\RapidR-2.121.0-win7-x86-alpha1\tests\results.txt' results.txt
```
The agent runs as SYSTEM (session 0, no desktop): console programs work, windows and Studio need the logged-in user's
desktop (start them through a scheduled task, `schtasks /create /ru <user> ... /it`, or look at them by hand). `utmctl` returns exit
status 0 even when the guest agent is missing: read its text.

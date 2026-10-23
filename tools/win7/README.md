# RapidR for Windows 7 / 8 / 8.1 (32 and 64 bit), built on the Mac

Lane WIN7-1, 2026-10-23. Background and the feasibility study: `docs/old-pcs-spike.md`.
These builds are **separate from the release** (stable Rust, Windows 10+): they use a
nightly Rust and the `*-win7-windows-gnu` targets, which run on Windows 7 and later.

| File | What it does |
|---|---|
| `build.sh [i686\|x86_64] [cargo args]` | builds `rapidr.exe` (default `-p rapidr-cli`) |
| `imports.py rapidr.exe` | lists the DLL functions the .exe needs at start, flags what Windows 7 lacks |
| `stage.sh [i686\|x86_64]` | the test drop: stripped exe, Studio's files, tests, READ-ME, a `.zip`, and the disc image (ISO) |
| `serve.sh` | serves the drop to the VM over the **host-only** network only (HTTP, read-only) |
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
- The `windows` / `windows-sys` crates import five functions Windows 7 lacks
  (`RoGetActivationFactory`, `RoOriginateErrorW`, `PathCchStripPrefix`, `CoIncrementMTAUsage`, and
  `combase.dll` as a whole). `build.sh` builds with `--cfg windows_raw_dylib`;
  `crates/rapidr-cli/build.rs` asks the linker to delay-load those DLLs on Windows 7 builds;
  `crates/rapidr-cli/src/win7.rs` answers for them if they are missing (ole32 instead of combase; "not
  available" for the others). On Windows 8 and later nothing changes.
- LLVM-MinGW has no `libgcc`: `build.sh` makes empty/alias `libgcc.a`, `libgcc_eh.a`, `libgcc_s.a` (compiler-rt
  and libunwind do the work) in the target folder.

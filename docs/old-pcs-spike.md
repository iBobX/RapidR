# Old PCs (Windows 7 / 8 / 8.1, 32 and 64 bit): feasibility spike

Date: 2026-10-23. Lane WIN7-SPIKE. Short answer: **it is feasible, and closer than
expected.** The whole of `rapidr-cli` (the compiler, the VM, the runtime, the UI
kernel and RapidR Studio's host) passes `cargo check` for the Windows 7 targets,
32 and 64 bit, with **one 3-line fix**. Nothing was linked or run yet: that needs a
linker we don't have on this Mac (see "The path to a first .exe").

## What was used

- `rustc 1.100.0-nightly (6eeff9a52 2026-09-23)`, from rustup, plus `rust-src`
  (`rustup component add rust-src --toolchain nightly`). The default toolchain
  (stable 1.98.1) was not touched. Tier-3 targets have no prebuilt standard
  library, so every build uses `-Z build-std=std,panic_abort`.
- Targets that exist in this nightly (`rustc +nightly --print target-list | grep win7`):
  `i686-win7-windows-msvc`, `x86_64-win7-windows-msvc`, `i686-win7-windows-gnu`,
  `x86_64-win7-windows-gnu`. All four run on Windows 7, 8, 8.1, 10 and 11. (Rust's
  normal Windows targets need Windows 10 since 1.78; this is the only way to old PCs.)
- Command (3 jobs, build on the external drive):
  `CARGO_BUILD_JOBS=3 CARGO_TARGET_DIR=/Volumes/RapidRBuild/wt-win7-spike/target cargo +nightly check -Z build-std=std,panic_abort --target i686-win7-windows-msvc -p <crate>`
- `cargo check` only type-checks. It proves the Rust compiles for the target. It does
  **not** prove the program starts or works on Windows 7.
- The one C dependency (SQLite, `libsqlite3-sys`, bundled) needs a C compiler for the
  target. On this Mac there is none, so for the check only, two do-nothing stand-in
  scripts played "compiler" and "lib.exe" (`CC_<target>` / `AR_<target>` environment
  variables, kept outside the repo). The C code was therefore **not** compiled.

## Results

| Crate | 32-bit `i686-win7-windows-msvc` | First blocker | Likely fix |
|---|---|---|---|
| rapidr-lexer | compiles | none | none |
| rapidr-parser | compiles | none | none |
| rapidr-bcgen (the compiler) | compiles | none | none |
| rapidr-vm | compiles | none | none |
| rapidr-runtime-core (all default features: GUI, network, database, audio, gamepad, FFI) | **fails**, then compiles | (1) `libsqlite3-sys` build script: no C compiler for the target. (2) after that, a real 32-bit error in `rapidr-ui-host-winit` (below) | (1) a C toolchain for the target (see below). (2) the 3-line fix below |
| rapidr-vm-host-native | compiles | none | none |
| rapidr-cli (= Studio's host; Studio itself is the `ide/*.rr` bytecode it runs) | compiles | none | none |

Other targets, same command on `rapidr-cli`:

| Target | Result |
|---|---|
| `x86_64-win7-windows-msvc` | compiles, **without** the 32-bit fix (the problem only exists on 32 bit) |
| `i686-win7-windows-gnu` | compiles (with the fix) |

(`x86_64-win7-windows-gnu` was not run separately; it is the 64-bit twin of the two
rows above.)

### The one real 32-bit error

`crates/rapidr-ui-host-winit/src/tray.rs:359` (the system-tray icon). On 32-bit
Windows the Windows header struct `NOTIFYICONDATAW` is `packed`, and rustc refuses
to take a reference into a packed field (`E0793`, "reference to field of packed struct
is unaligned"). The fix: fill a local array and copy it in. It was tried here and
reverted so no production code changed:

```diff
-                for (d, s) in data.szTip.iter_mut().zip(item.shown.0.encode_utf16().take(127)) {
+                let mut tip = [0u16; 128];
+                for (d, s) in tip.iter_mut().zip(item.shown.0.encode_utf16().take(127)) {
                     *d = s;
                 }
+                data.szTip = tip;
```

It is also fine on 64-bit and every other platform.

### Three things that looked risky and are not

- **Calls into DLLs (`DECLARE`)** go through `extern "system"` functions
  (`rapidr-runtime-core/src/ffi.rs`). On 32-bit Windows "system" is `stdcall`, which is
  exactly what RapidQ programs use. A 32-bit build should be *more* faithful to old
  RapidQ programs than the 64-bit one, but it must be tested (see tests below).
- **Windows APIs RapidR itself calls** (`windows-sys` uses in `rapidr-ui-host-winit`,
  `rapidr-runtime-core`, `rapidr-value`): all are Windows 2000 / Vista era
  (`Shell_NotifyIconW`, `RegGetValueW`, `SetWindowSubclass`, GDI, console, serial...).
  None of the Windows 8/10-only functions came up.
- **Third-party crates** (a text search over about 270 crates in the tree for
  `GetSystemTimePreciseAsFileTime`, `WaitOnAddress`, `SetThreadDescription`,
  `CreatePseudoConsole`, `DwmSetWindowAttribute`, `GetDpiForWindow`,
  `GetFinalPathNameByHandle`, `GetTickCount64` and 20 more): every hit is loaded at
  run time with a fallback (`winit` says "Windows 7 or higher"; `parking_lot_core`
  falls back to keyed events; `muda`, `schannel` use `GetProcAddress`). The
  Windows-7 version of Rust's own standard library also avoids the new calls.
  This is a name search, not a proof. The proof is the import table of the linked .exe.

### What will behave differently on an old PC (found by reading, to be confirmed by running)

1. **Graphics.** wgpu 30 has Vulkan, DirectX 12, OpenGL (no DirectX 11). Windows 7 has no
   DirectX 12, and many old PCs have no Vulkan driver. RapidR's desktop host already
   handles this: `software_gpu_only` in `rapidr-ui-host-winit/src/winit_host.rs` picks the
   **CPU renderer (vello_cpu drawn to the window with softbuffer / plain GDI)** when wgpu
   finds no real GPU on Vulkan / DX12 / Metal. That is the same renderer the web and the
   captures use, so there is nothing new to write. A PC with a Windows-7-era Nvidia/AMD
   Vulkan driver gets the GPU path. `RAPIDR_RENDERER=cpu` forces the CPU one for testing.
   wgpu loads `d3d12.dll` and `d3dcompiler_47.dll` at run time (neither is on Windows 7),
   so their absence gives "no adapter", not a failed start; `dxgi.dll` exists on Windows 7.
2. **CPU.** Both 32-bit targets assume a Pentium 4 (SSE2). That excludes only CPUs
   without SSE2 (Pentium III, Athlon XP and older). The 64-bit ones assume plain x86-64.
   `vello_cpu` (through `fearless_simd`) picks its SIMD level at run time.
3. **Console programs: colours and `CLS` / `LOCATE` / `COLOR`.**
   `rapidr-runtime-core/src/terminal.rs` switches on `ENABLE_VIRTUAL_TERMINAL_PROCESSING`.
   Windows 7, 8 and 8.1 ignore it (the call fails, harmlessly), so the escape codes
   would print as text. Needs a small fallback with the old console calls
   (`SetConsoleTextAttribute`, `SetConsoleCursorPosition`...). A real gap, not a blocker.
4. **Fonts.** `fontique` asks DirectWrite for the system fonts; the Windows 8.1-only
   interface (`IDWriteFactory2`, used for emoji / script fallback) is cast and may be
   missing on Windows 7, so emoji / CJK fallback may look different. Studio's own fonts
   are bundled and unaffected. To check by eye.
5. **Screen readers.** AccessKit uses UI Automation (Windows 7 SP1 has it). To check.
6. **32-bit memory.** About 2 GB for a normal process on 32-bit Windows (rustc adds
   `/LARGEADDRESSAWARE` and `/SAFESEH` for the msvc target: up to 4 GB on 64-bit Windows).
   Big RDataFrame work (millions of rows) will hit that. Studio itself is fine.
7. **C runtime.** With MSVC, link with `+crt-static` so the old PC needs no Visual C++
   redistributable. With MinGW use the **msvcrt** flavour (the UCRT flavour needs
   Microsoft's KB2999226 update on Windows 7).

## Linking: what we have and what we need

On this Mac a link fails at once:
`i686-w64-mingw32-gcc not found` (gnu targets) and `link.exe not found` (msvc targets).
rustup's nightly does ship `rust-lld` and `lld-link`, but the msvc route also needs
Microsoft's Windows SDK libraries (`kernel32.lib`, `advapi32.lib`, ...): the only
Mac way is a tool that downloads them from Microsoft (cargo-xwin). Not recommended
(licence terms, and it is not how RapidR links today).

| Option | Works from | Needs | Notes |
|---|---|---|---|
| **A. LLVM-MinGW + the `*-win7-windows-gnu` target** | the Mac (or the Windows VM) | LLVM-MinGW, **msvcrt** flavour, 32 and 64 bit (about 100 MB, Apache-2.0 + LLVM exception) | Same linker family RapidR already ships and audits (`docs/release-packaging.md`, `tools/release/windows/link_audit.sh`). Rust's `-gnu` targets expect GNU names (`i686-w64-mingw32-gcc`, `libgcc_eh`); set `CARGO_TARGET_I686_WIN7_WINDOWS_GNU_LINKER` to LLVM-MinGW's clang, plus `CC_` / `AR_` for SQLite. The `libgcc_eh` / unwinder names are the one thing not verified; with `panic=abort` it should be small. |
| **B. MSVC Build Tools + the `*-win7-windows-msvc` target** | the Windows 11 ARM VM | Visual C++ Build Tools with the x86 and x64 tools (the VM may already have them) | The most "standard" result. Free for individuals and small organisations. Compiles SQLite with `cl.exe`. |
| C. mingw-w64 GCC (Homebrew) | the Mac | `brew install mingw-w64` (GPL compiler, runtime exception) | Works the same way as A, but a GPL toolchain; RapidR's policy prefers A. |

## The path to a first .exe for the Windows 7 VM

1. **Get a linker (about half a day, mostly download and trying).** Option A on the Mac:
   LLVM-MinGW (msvcrt) next to the existing release tooling. Option B instead if the
   Windows VM is free: `rustup toolchain install nightly --profile minimal`,
   `rustup component add rust-src --toolchain nightly`, MSVC Build Tools.
2. **Land the tray fix (10 minutes)** and make sure `cargo check` stays green for the
   32-bit target in `tools/regress.sh` (a new, optional, nightly-only stage; the stable
   toolchain stays the release toolchain). Pin the nightly by date in `release.toml`.
3. **Build the console program first (about 1 day including surprises):**
   `cargo +nightly build --release -Z build-std=std,panic_abort --target x86_64-win7-windows-gnu -p rapidr-cli`
   (64-bit first: it needed no code change), then the same for `i686-win7-windows-gnu`.
   Expect the first link to need a flag or two (`+crt-static`, the C runtime flavour).
4. **Check the .exe before copying it:** `llvm-objdump -p rapidr.exe` lists the DLL
   functions it imports; compare against Windows 7's DLLs (on the VM:
   `dumpbin /exports kernel32.dll`). Any missing name is a "procedure entry point not
   found" at start. This is the real proof of the name search above.
5. **Run it on the Windows 7 VM (1-3 days of fixes):** `rapidr.exe run hello.bas`, then
   a form program with `RAPIDR_RENDERER=cpu`, then Studio. Walk the items under
   "What will behave differently" (console colours first). Run the Windows-specific
   suites there (FFI, serial, registry, `tests/` that apply).
6. **Decide the product shape (with Robert).** Probably a separate "RapidR for older
   Windows" download (one installer per architecture, nightly-built, labelled as
   such), with one honest line in the release notes; the main Windows download stays
   on stable Rust.

Rough total to a *running Studio* on Windows 7: **about 3 to 6 working days**, of which
only the first 1-2 are certain. The unknowns are all at run time (items 1, 3, 4, 5 above),
none in the Rust code itself.

## Decisions for Robert

- Which Windows 7 VM is it (32 or 64 bit)? 64-bit machines are probably the majority, but
  the community request was Windows 8.1 **32-bit**, so both are on the plan; they differ
  only by the tray fix and by what the tests find.
- Linker: LLVM-MinGW on the Mac (A) or the Windows VM (B)? A fits the current release
  tooling; B is simplest if the VM is free.
- Per the "lawyer agent for every new thing" rule: LLVM-MinGW is already approved; the
  **msvcrt flavour** and the nightly `build-std` standard library (MIT / Apache-2.0, plus
  `compiler_builtins`, `hashbrown`, `gimli`, `addr2line`, `rustc-demangle` from Rust's own
  lock) should get the usual one-line licence check before anything ships.

## Disk used

`/Volumes/RapidRBuild/wt-win7-spike/target` is **3.3 GB** (std for three targets plus the
dependencies, check-only). Delete it with `rm -rf /Volumes/RapidRBuild/wt-win7-spike`
once the follow-up lane no longer needs it. Nothing was written to the internal disk
besides rustup's `rust-src` (about 80 MB, in `~/.rustup`).

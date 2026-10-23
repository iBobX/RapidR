# Legal review, 2026-10-23: the Windows 7 / 8 / 8.1 (32-bit) alpha download

This is the "lawyer agent" review (a standing rule since 2026-10-22) for what lane WIN7-ALPHA puts in `RapidR-2.121.0-win7-x86-alpha1.zip`. It is a careful risk review of what the linker puts into `rapidr.exe` and which licence texts travel with it, **not legal advice**. Method: the program was linked with a linker map (`tools/win7/link_audit.sh`, which reuses `tools/release/windows/member_licences.py`), every toolchain object the link pulls in was traced to its mingw-w64 source file at the commit LLVM-MinGW names, and each file's header was read.

**Result: nothing GPL or LGPL is linked, and nothing in the zip is GPL or LGPL.** One Cephes-derived pair (`cbrt`, `cbrtf`) was linked and has been replaced by RapidR's own code (below). One item is flagged for Robert's decision (Microsoft-derived delay-load helper, Low).

## What the build is

| | |
|---|---|
| Program | `rapidr.exe`, `i686-win7-windows-gnu`, release, RapidR 2.121.0 |
| Rust | nightly 1.100.0 (`6eeff9a52`, 2026-09-23) with `-Z build-std=std,panic_abort` (the Windows 7 targets are tier 3: no prebuilt standard library) |
| Linker and C compiler | LLVM-MinGW 20261020 with LLVM 23.1.4: the macOS release's tools plus the **msvcrt** sysroot of the same release (Windows 7 has `msvcrt.dll`, not the Universal CRT) |
| mingw-w64 | commit `0d6dfbc348dfad714761c0a7151615767f96bcb5` (`MINGW_W64_VERSION` in LLVM-MinGW 20261020's `build-mingw-w64.sh`; reports itself as 15.0.0 "alpha") |
| Official source | https://sourceforge.net/p/mingw-w64/mingw-w64/ (the project's own repository); the GitHub mirror `mingw-w64/mingw-w64` has the same commit, and its five licence files are byte-identical to the SourceForge ones (checked by SHA-256) |

## Linked into `rapidr.exe`

| # | Component | Licence | What it asks | Where the text is in the zip |
|---|---|---|---|---|
| 1 | RapidR (`rapidr-cli` and its 34 workspace crates) | MIT, Ruta Internet SRL | the notice | `LICENSE`, `NOTICE`, `LEGAL.md` |
| 2 | 283 crates.io libraries (the i686-win7 dependency graph, `cargo +nightly tree --target i686-win7-windows-gnu`) | the allowlist of `deny.toml` (MIT, Apache-2.0, BSD, ISC, Zlib, 0BSD, BSL-1.0, Unlicense, Unicode, CC0: no copyleft of any strength) | notices | all 283 are in `THIRD_PARTY_NOTICES.md` (script check, 283 of 283); texts in `LICENSES.md` |
| 3 | Rust's standard library (core, alloc, std, panic_abort, unwind and the crates they are built from: compiler_builtins 0.1.160, hashbrown, memchr, gimli, object, addr2line, miniz_oxide, adler2, rustc-demangle, cfg-if, libc, windows-sys) | MIT OR Apache-2.0 (memchr: Unlicense OR MIT; miniz_oxide: MIT OR Zlib OR Apache-2.0; adler2: 0BSD OR MIT OR Apache-2.0; compiler_builtins' compiler-rt parts: Apache-2.0 WITH LLVM-exception) | notice (MIT); the LLVM exception waives it for compiled-in code | `licenses\llvm\LICENSE.TXT`; the MIT/Apache notices of std's crates are the ones every RapidR build carries (`notices.rs`, "Rust standard library"), which this zip has no way to print (see 5) |
| 4 | SQLite (bundled C, compiled by LLVM-MinGW's clang), the embedded fonts (OFL-1.1) | public domain; OFL-1.1 | the font notices | `LICENSES.md` (unchanged from every other RapidR build) |
| 5 | mingw-w64 start-up objects `crt2.o`, `crtbegin.o`, `crtend.o`; `libmingw32` (15 members: `CRT_fp10`, `_newmode`, `cinitexe`, `dllargv`, `gccmain`, `merr`, `mingw_helpers`, `natstart`, `pseudo-reloc`, `pseudo-reloc-list`, `usermatherr`, `wildcard`, `xncommod`, `xthdloc`, `xtxtmode`) | public domain; `cinitexe` ZPL-2.1 | the notices go with binary copies of the ZPL parts | `licenses\mingw-w64\COPYING` (ZPL-2.1), `DISCLAIMER.PD` |
| 6 | `libmingwex` (20 members): the delay-load helper (`delayimp`, `delay-f`, `delay-n`), x87 math (`cosl_internal`, `sinl_internal`, `exp2l`, `log2l`, `internal_logl`, `powi`, `ldexp`), `hypotf`, `mingw_matherr`, Control Flow Guard and load-config support, `mingw_fprintf` and `mingw_pformat` (Keith Marshall: "without restriction of copyright"), and David M. Gay's `gdtoa`, `dmisc`, `gmisc`, `misc` | public domain; ZPL-2.1 (`hypotf`, `mingw_matherr`, `powi`); Lucent/David M. Gay permission notice (HPND) for the four gdtoa files | the gdtoa notice must appear with binary copies | `licenses\mingw-w64\COPYING.MinGW-w64-runtime.txt` (the gdtoa section), `COPYING` |
| 7 | `libmsvcrt_common` (52 members) and `libmsvcrt_extra` (11): math wrappers around `msvcrt.dll` (`sin`, `cos`, `exp`, `pow`, `log`, `atan2f`, ...) and C-runtime shims | ZPL-2.1 (`acosh`, `acoshf`, `cos`, `exp`, `log`, `pow`, `sin` carry the full ZPL header; `hypot`, `invalid_parameter_handler`, `mingw_lock`, `strnlen`, `wcsnlen` have none and fall under mingw-w64's `COPYING`); public domain for the rest. `acosf`, `asinf`, `atan2f`, `atanf`, `fmodf` are J.T. Conklin's NetBSD code, marked "Public domain" in the file | as above | as above |
| 8 | LLVM libunwind (`libunwind`, `UnwindLevel1`, `UnwindLevel1-gcc-ext`, `UnwindRegistersRestore`, `UnwindRegistersSave`) | Apache-2.0 WITH LLVM-exception | nothing for compiled-in code (§4(a), (b), (d) waived); the text is shipped anyway | `licenses\llvm\LICENSE.TXT` |
| 9 | The program's own version information and manifest (`crates/rapidr-cli/build.rs`, compiled by `llvm-windres`) | RapidR's (MIT) | — | — |
| 10 | Windows' own DLLs the program imports (`kernel32`, `user32`, `msvcrt.dll`, ...) | part of Windows | nothing: not in the zip | — |

106 toolchain objects in all: 52 `libmsvcrt_common`, 20 `libmingwex`, 15 `libmingw32`, 11 `libmsvcrt_extra`, 3 start-up objects, 5 libunwind. After the fix below the audit prints **0 flagged**.

**Not linked** (checked in the map): winpthreads, compiler-rt's builtins (Rust's `compiler_builtins` provides them), libgcc, `libgmon` (profiling, only with `-pg`), Cephes-derived code, anything imported from Wine. The only GPL/LGPL text in `mingw-w64-crt` is in its build scripts (`build-aux`) and `profile/` (gmon), none of it linked.

## Findings

| # | Item | Risk | The catch | Action | Status |
|---|---|---|---|---|---|
| 1 | **`cbrt` and `cbrtf` came from Cephes** | Medium before, none now | `msvcrt.dll` has no cube root, so the linker took mingw-w64's `math/cbrt.c` and `cbrtf.c`, which include `cephes_mconf.h` (Stephen Moshier's Cephes library; `COPYING.MinGW-w64-runtime.txt` itself says its permission is "vague"). The files are marked public domain by mingw-w64, but the original author's permission is not clear, and the project's rule (docs/licensing.md §3.3) is that Cephes-derived code is not linked. The Universal-CRT builds never hit this (the UCRT DLL has `cbrt`). | RapidR defines its own `cbrt` and `cbrtf` (`crates/rapidr-cli/src/cbrt.rs`: an exponent-bit first guess and Newton's iteration; tested to within 1 ulp of the platform's across the whole range, exact for whole cubes). The linker takes an object's own definition before an archive member, so the library's never gets in (`frexp` went with it). Verified in the map: no `cbrt`/`cbrtf` member. | **Done** |
| 2 | **`delayimp.c` is derived from Microsoft's `delayhlp.cpp`** | Low | The Windows 7 build delay-loads the five functions Windows 8 lacks (`crates/rapidr-cli/src/win7.rs`), which needs mingw-w64's delay-load helper. Its header says: public domain, "derived from Microsoft implementation file delayhlp.cpp, which is free for users to modify and derive". I could not check Microsoft's side offline; the claim is the mingw-w64 project's, shared by every program built with MinGW that delay-loads a DLL. | Keep for the alpha. Before the proper download: replace it with a helper of our own (about 100 lines in `win7.rs`: resolve the DLL and function, patch the import slot, call our fallback), and re-test on Windows 7. | **Decision for Robert** (recommended: replace before the non-alpha download) |
| 3 | `COPYING.MinGW-w64-runtime.txt` also lists LGPL Wine headers/IDLs | None | The Wine-imported files are headers and IDLs, compiled against (by SQLite's C and the `cc` crate), never linked; nothing in `mingw-w64-crt` that is linked is imported from Wine. Using LGPL *headers* in a compile does not make the program LGPL (LGPL-2.1 §6 / 3: numerical parameters, data structure layouts, small macros). | Say so (this table). The zip carries the whole runtime notice file anyway. | Done |
| 4 | The toolchain archive holds `gendef` (GPL) and `widl` (LGPL) | None | Build tools in LLVM-MinGW's `share/`, never run by `build.sh` and never linked; nothing from the toolchain tree ships except what the linker copies into the `.exe`. | Keep it that way: the zip is assembled from an explicit list (`tools/win7/alpha.sh`). | Done |
| 5 | The alpha folder cannot print `rapidr notices`, and the notice generator describes the *UCRT* builds | Low | `rapidr notices` reads an install's `release.toml` and `notices/` (or generates them from a checkout's `Cargo.lock`); the alpha folder is neither, and an install's `release.toml` would make Studio look for `ide/rapidr-ide.rrbc` instead of the source. `notices.rs` also names the mingw-w64 members of the `*-pc-windows-gnullvm` (UCRT) builds; the licence set is the same for the msvcrt build (public domain, ZPL-2.1, HPND), but the member names differ. | For the alpha, the zip carries the notice files as plain files (`THIRD_PARTY_NOTICES.md`, `LICENSES.md`, `licenses\`, this audit). For the proper download, give the Windows 7 build an install layout with its own `notices/windows-x86.txt` and update `extras()`. | Open, harmless for the alpha |
| 6 | An unsigned program | Low | Windows SmartScreen and antivirus programs distrust new unsigned programs. The 2.121.0 test saw Bitdefender hold resource-less x64 programs; `rapidr.exe` now carries version information (company "Ruta Internet S.R.L.", product "RapidR", version 2.121.0) and a manifest. | `README-ALPHA.txt` says it is unsigned and how to run it anyway ("More info", "Run anyway"), never to switch protection off. Signing is Robert's decision (docs/legal/review-2026-10-22.md, 7c). | Done |
| 7 | Alpha wording | None | "Compatible with RapidQ" only; no "clone". | Checked in `README-ALPHA.txt`. | Done |

The six traps (CLAUDE.md): not triggered. The zip has no sign-up, no analytics, no outside fonts, scripts or CDNs, no uploads and no emails; the AI helper is off until a person types their own key, which this alpha keeps in memory only.

## Redoing the audit

```sh
# mingw-w64 at commit 0d6dfbc3… (SourceForge, or its GitHub mirror), extracted
tools/win7/link_audit.sh <folder holding mingw-w64-crt/> [work folder]
```

It relinks with a map, lists the toolchain's objects and exits 1 on a flag; then it builds the program the usual way again.

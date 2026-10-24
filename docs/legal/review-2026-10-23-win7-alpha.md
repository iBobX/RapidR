# Legal review, 2026-10-23: the Windows 7 / 8 / 8.1 (32-bit) alpha download

This is the "lawyer agent" review (a standing rule since 2026-10-22) for what lane WIN7-ALPHA puts in `RapidR-2.121.0-win7-x86-alpha1.zip`. It is a careful risk review of what the linker puts into `rapidr.exe` and which licence texts travel with it, **not legal advice**. Method: the program was linked with a linker map (`tools/win7/link_audit.sh`, which reuses `tools/release/windows/member_licences.py`), every toolchain object the link pulls in was traced to its mingw-w64 source file at the commit LLVM-MinGW names, and each file's header was read.

**Result: nothing GPL or LGPL is linked, and nothing in the zip is GPL or LGPL.** One Cephes-derived pair (`cbrt`, `cbrtf`) was linked and has been replaced by RapidR's own code (below). One item was flagged for Robert's decision (Microsoft-derived delay-load helper, Low); the addendum below ranked it Medium and it is now gone (lane WIN7-ALPHA-2).

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
| 6 | `libmingwex` (17 members; 20 before the delay-load helper `delayimp`, `delay-f`, `delay-n` was removed, see the addendum's status below): x87 math (`cosl_internal`, `sinl_internal`, `exp2l`, `log2l`, `internal_logl`, `powi`, `ldexp`), `hypotf`, `mingw_matherr`, Control Flow Guard and load-config support, `mingw_fprintf` and `mingw_pformat` (Keith Marshall: "without restriction of copyright"), and David M. Gay's `gdtoa`, `dmisc`, `gmisc`, `misc` | public domain; ZPL-2.1 (`hypotf`, `mingw_matherr`, `powi`); Lucent/David M. Gay permission notice (HPND) for the four gdtoa files | the gdtoa notice must appear with binary copies | `licenses\mingw-w64\COPYING.MinGW-w64-runtime.txt` (the gdtoa section), `COPYING` |
| 7 | `libmsvcrt_common` (52 members) and `libmsvcrt_extra` (11): math wrappers around `msvcrt.dll` (`sin`, `cos`, `exp`, `pow`, `log`, `atan2f`, ...) and C-runtime shims | ZPL-2.1 (`acosh`, `acoshf`, `cos`, `exp`, `log`, `pow`, `sin` carry the full ZPL header; `hypot`, `invalid_parameter_handler`, `mingw_lock`, `strnlen`, `wcsnlen` have none and fall under mingw-w64's `COPYING`); public domain for the rest. `acosf`, `asinf`, `atan2f`, `atanf`, `fmodf` are J.T. Conklin's NetBSD code, marked "Public domain" in the file | as above | as above |
| 8 | LLVM libunwind (`libunwind`, `UnwindLevel1`, `UnwindLevel1-gcc-ext`, `UnwindRegistersRestore`, `UnwindRegistersSave`) | Apache-2.0 WITH LLVM-exception | nothing for compiled-in code (§4(a), (b), (d) waived); the text is shipped anyway | `licenses\llvm\LICENSE.TXT` |
| 9 | The program's own version information and manifest (`crates/rapidr-cli/build.rs`, compiled by `llvm-windres`) | RapidR's (MIT) | — | — |
| 10 | Windows' own DLLs the program imports (`kernel32`, `user32`, `msvcrt.dll`, ...) | part of Windows | nothing: not in the zip | — |

103 toolchain objects in all (106 before the delay-load helper went): 52 `libmsvcrt_common`, 17 `libmingwex`, 15 `libmingw32`, 11 `libmsvcrt_extra`, 3 start-up objects, 5 libunwind. After the fix below the audit prints **0 flagged**.

**Not linked** (checked in the map): winpthreads, compiler-rt's builtins (Rust's `compiler_builtins` provides them), libgcc, `libgmon` (profiling, only with `-pg`), Cephes-derived code, anything imported from Wine. The only GPL/LGPL text in `mingw-w64-crt` is in its build scripts (`build-aux`) and `profile/` (gmon), none of it linked.

## Findings

| # | Item | Risk | The catch | Action | Status |
|---|---|---|---|---|---|
| 1 | **`cbrt` and `cbrtf` came from Cephes** | Medium before, none now | `msvcrt.dll` has no cube root, so the linker took mingw-w64's `math/cbrt.c` and `cbrtf.c`, which include `cephes_mconf.h` (Stephen Moshier's Cephes library; `COPYING.MinGW-w64-runtime.txt` itself says its permission is "vague"). The files are marked public domain by mingw-w64, but the original author's permission is not clear, and the project's rule (docs/licensing.md §3.3) is that Cephes-derived code is not linked. The Universal-CRT builds never hit this (the UCRT DLL has `cbrt`). | RapidR defines its own `cbrt` and `cbrtf` (`crates/rapidr-cli/src/cbrt.rs`: an exponent-bit first guess and Newton's iteration; tested to within 1 ulp of the platform's across the whole range, exact for whole cubes). The linker takes an object's own definition before an archive member, so the library's never gets in (`frexp` went with it). Verified in the map: no `cbrt`/`cbrtf` member. | **Done** |
| 2 | **`delayimp.c` is derived from Microsoft's `delayhlp.cpp`** | Low | The Windows 7 build delay-loads the five functions Windows 8 lacks (`crates/rapidr-cli/src/win7.rs`), which needs mingw-w64's delay-load helper. Its header says: public domain, "derived from Microsoft implementation file delayhlp.cpp, which is free for users to modify and derive". I could not check Microsoft's side offline; the claim is the mingw-w64 project's, shared by every program built with MinGW that delay-loads a DLL. | Keep for the alpha. Before the proper download: replace it with a helper of our own (about 100 lines in `win7.rs`: resolve the DLL and function, patch the import slot, call our fallback), and re-test on Windows 7. | **Done** (lane WIN7-ALPHA-2): replaced by RapidR's own code, see the status below |
| 3 | `COPYING.MinGW-w64-runtime.txt` also lists LGPL Wine headers/IDLs | None | The Wine-imported files are headers and IDLs, compiled against (by SQLite's C and the `cc` crate), never linked; nothing in `mingw-w64-crt` that is linked is imported from Wine. Using LGPL *headers* in a compile does not make the program LGPL (LGPL-2.1 §6 / 3: numerical parameters, data structure layouts, small macros). | Say so (this table). The zip carries the whole runtime notice file anyway. | Done |
| 4 | The toolchain archive holds `gendef` (GPL) and `widl` (LGPL) | None | Build tools in LLVM-MinGW's `share/`, never run by `build.sh` and never linked; nothing from the toolchain tree ships except what the linker copies into the `.exe`. | Keep it that way: the zip is assembled from an explicit list (`tools/win7/alpha.sh`). | Done |
| 5 | The alpha folder cannot print `rapidr notices`, and the notice generator describes the *UCRT* builds | Low | `rapidr notices` reads an install's `release.toml` and `notices/` (or generates them from a checkout's `Cargo.lock`); the alpha folder is neither, and an install's `release.toml` would make Studio look for `ide/rapidr-ide.rrbc` instead of the source. `notices.rs` also names the mingw-w64 members of the `*-pc-windows-gnullvm` (UCRT) builds; the licence set is the same for the msvcrt build (public domain, ZPL-2.1, HPND), but the member names differ. | For the alpha, the zip carries the notice files as plain files (`THIRD_PARTY_NOTICES.md`, `LICENSES.md`, `licenses\`, this audit). For the proper download, give the Windows 7 build an install layout with its own `notices/windows-x86.txt` and update `extras()`. | **Done** for the alpha (lane WIN7-ALPHA-2): the zip carries `THIRD-PARTY-NOTICES.txt` made by `rapidr notices`; the Windows 7 build's own install layout and `extras()` stay for the proper download |
| 6 | An unsigned program | Low | Windows SmartScreen and antivirus programs distrust new unsigned programs. The 2.121.0 test saw Bitdefender hold resource-less x64 programs; `rapidr.exe` now carries version information (company "Ruta Internet S.R.L.", product "RapidR", version 2.121.0) and a manifest. | `README-ALPHA.txt` says it is unsigned and how to run it anyway ("More info", "Run anyway"), never to switch protection off. Signing is Robert's decision (docs/legal/review-2026-10-22.md, 7c). | Done |
| 7 | Alpha wording | None | "Compatible with RapidQ" only; no "clone". | Checked in `README-ALPHA.txt`. | Done |

The six traps (CLAUDE.md): not triggered. The zip has no sign-up, no analytics, no outside fonts, scripts or CDNs, no uploads and no emails; the AI helper is off until a person types their own key, which this alpha keeps in memory only.

## Redoing the audit

```sh
# mingw-w64 at commit 0d6dfbc3… (SourceForge, or its GitHub mirror), extracted
tools/win7/link_audit.sh <folder holding mingw-w64-crt/> [work folder]
```

It relinks with a map, lists the toolchain's objects and exits 1 on a flag; then it builds the program the usual way again.

## Antivirus check (2026-10-23, 21:26)

`tools/win7/av_check.ps1`, in the Windows 11 ARM VM with Bitdefender real-time on (Defender off), from `C:\rapidr-win7-av` (outside Temp, which has an exception), after the release jobs there had finished: the zip matched its `.sha256`; all 149 files unzipped; `rapidr.exe` showed Company "Ruta Internet S.R.L.", Product "RapidR", version 2.121.0, "RapidR (Windows 7 alpha)"; `rapidr.exe version`, `hello.rr` and `language.rr` ran (exit code 0, 32-bit program under x86 emulation); Bitdefender's quarantine folder and `pending_actions.json` were unchanged afterwards. Bitdefender's settings were not touched; the folder was removed. Not covered: Studio and window programs (a session-0 script has no desktop), and other antivirus products.

## Addendum (lawyer agent, 2026-10-23 night)

Scope: (1) the zip `RapidR-2.121.0-win7-x86-alpha1.zip` (sha256 `1e5e162b…7ec6`, listed and read: README-ALPHA.txt, `licenses\`, LICENSES.md, THIRD_PARTY_NOTICES.md), and the open delay-load point; (2) the music of `rapidr-2.121.0-launch-intro-music.mp4`. Primary sources: mingw-w64 at commit `0d6dfbc3…` (headers of the linked files), Microsoft Learn ("Understand the delay load helper function"), the Visual Studio licence terms (Distributable Code clause, as quoted by Microsoft's own pages), and the Logic Pro licence PDF installed on this Mac (`/Applications/Logic Pro.app/Contents/Resources/Logic Pro License Agreement.pdf`, English part, document EA1924 dated 08/20/2024). US/EU/Uruguay common practice, not legal advice.

### Verdicts, ranked

| # | Risk | Item | Verdict | Action before publishing |
|---|---|---|---|---|
| 1 | Medium | mingw-w64 delay-load helper (`delayimp.o`, `delay-f.o`, `delay-n.o`) derived from Microsoft's `delayhlp.cpp` | **Replace** | Write our own `__delayLoadHelper2` (below), rebuild, re-test on Windows 7, re-run `link_audit.sh` (must show those three objects gone). |
| 2 | Medium | The zip lacks the full licence texts and copyright lines of the 283 crates and of Rust's standard library | **Change before publishing** | Put the generated `THIRD-PARTY-NOTICES.txt` into the zip (below). |
| 3 | Medium | Our Windows 7 test VM: unactivated Windows 7 from an archive.org ISO | **Change how we test** | Stop after the current round, delete the VM, move to community testers or a real PC with its COA (below). |
| 4 | Low | README-ALPHA.txt wording: no "as is / no warranty" sentence, no note that Windows 7 is out of support, no Microsoft trademark line, support line without the company name | **Change X** (4 sentences, below) | Edit README-ALPHA.txt (both languages). |
| 5 | Low | mingw-w64 `AUTHORS` file not in `licenses\mingw-w64\` | **Change X** | Add it (2.4 KB, same commit). The ZPL notice in `COPYING` points to it. |
| 6 | Low | Apple Loops in the launch video | **OK as is**, with conditions | Conditions in the music section. |
| 7 | Low | YouTube Content ID claim on Apple Loops | **OK**, handle as below | Upload unlisted first; dispute with the licence if claimed. |
| 8 | None | Rust nightly `-Z build-std` | **OK as is** | Nothing beyond item 2. |
| 9 | None | Names "win7" / "Windows 7" | **OK as is** | Only the trademark line of item 4. |
| 10 | None | Rest of the linked mingw-w64 objects and the licence files | **OK as is** | Only item 5. |

Nothing is High. Items 1 to 3 are Medium because the rule here is "not clearly safe, replace", not because anyone has ever complained.

### 1. Delay-load helper: replace

What I found.
- `delayimp.c` (278 lines, mingw-w64 `misc/`) says "no copyright assigned … Public Domain" and "derived from Microsoft implementation file delayhlp.cpp, which is free for users to modify and derive". `delay-f.c` and `delay-n.c` (13 lines each) only define the two hook pointers (`__pfnDliFailureHook2`, `__pfnDliNotifyHook2`), public domain, no Microsoft text. No other linked mingw-w64 file mentions Microsoft code (grepped the start-up, `libmingw32`, `libmingwex` sources; `mingw_pformat`/`mingw_fprintf` only mention Microsoft's *behaviour*).
- Microsoft's side. `delayhlp.cpp` ships in the MSVC `include` directory. Microsoft Learn says you "can modify the helper function", should "write your own function", and that the default helper "is a good template to use as an example" and "you can use the existing code as a guide". That is permission to *write* a helper, not a licence to copy the file, and it says nothing about redistributing a derivative or relicensing it as public domain. The Visual Studio licence terms grant source-level rights only to files "marked as sample/template" and bind "Distributable Code" (object code from the REDIST list) with conditions, including no "Excluded License" (a licence that forces disclosure or open modification). I found no text that puts `delayhlp.cpp` on either list, and I could not read its header offline. So the "free to modify and derive" sentence is the mingw-w64 project's claim, not a Microsoft licence we can point to.
- Practical risk is small (every MinGW program that delay-loads has carried this file for about 15 years, and nobody has complained), but the cost of removal is small too (about 100 lines), and Robert's rule is to replace what is not clearly safe. "Ship as is for the alpha" is therefore not recommended. If the replacement cannot be built and re-tested on Windows 7 in time, the alpha slips; it does not ship with this file.

How to replace it (clean-room, from public specifications only).
1. Write the replacement with an agent that has **not** read `delayimp.c` or `delayhlp.cpp`. Sources allowed: Microsoft's PE format specification, section "Delay-Load Import Tables" (the delay-load directory layout: attributes, name RVA, module-handle RVA, delay IAT RVA, delay name table RVA, and the import name table entries), and the Learn page above for the helper's *calling contract* (`FARPROC WINAPI __delayLoadHelper2(PCImgDelayDescr pidd, FARPROC *ppfnIATEntry)`: resolve the function, store it in the IAT slot, return it). The thunks that call it are generated by LLVM's lld (Apache-2.0 with LLVM exception), not by mingw-w64.
2. Put it in `crates/rapidr-cli/src/win7.rs` as `#[no_mangle] extern "system" fn _delayLoadHelper2@8` (`__delayLoadHelper2` on i686 with stdcall decoration), and fold the existing hook logic (already ours: combase.dll read from ole32.dll, stand-ins for the functions Windows 7 lacks) straight into it, so `__pfnDliFailureHook2` is no longer needed. An object's own definition beats an archive member, the same trick that removed Cephes' `cbrt`.
3. Simpler alternative with no PE parsing at all: give up `--delayload` and define the six functions yourself (`RoGetActivationFactory`, `RoOriginateErrorW`, `PathCchStripPrefix`, `CoIncrementMTAUsage`, and the two `combase.dll` ones, `CoCreateFreeThreadedMarshaler`, `CoTaskMemFree`) as small shims that call `GetProcAddress` on first use and fall back to the stand-ins. Either is fine; the first keeps today's linking.
4. Add one paragraph to `docs/legal/clean-room.md` (who wrote it, from which sources, that `delayimp.c`/`delayhlp.cpp` were not consulted). Update item 6 of the licence table above and `licenses\README.txt` ("the delay-loading helper" disappears from the mingw-w64 list). Re-run `tools/win7/link_audit.sh`.

### 2. Notices for the 283 crates and Rust's standard library

The zip carries `THIRD_PARTY_NOTICES.md` (names, versions, links) and `LICENSES.md` (one MIT text, fonts, the fdlibm/Sun, NetBSD and similar notices) but not the full licence text with each crate's copyright line. MIT, BSD, ISC, Zlib and Apache-2.0 all ask for the notice to travel with *binary* copies, and the standard library (MIT OR Apache-2.0, plus the crates it is built from) is in the same position. The normal downloads solve this with the `THIRD-PARTY-NOTICES.txt` that `rapidr notices` generates from the real dependency graph (`crates/rapidr-cli/src/notices.rs`); the alpha folder is not an install, so finding 5 above ("harmless") is **not** right for a public download.
- Action: generate that file for the win7 graph (or, if the generator cannot take the win7 target, use the Windows x64 one, whose crate set is a superset: extra notices are harmless) and put it in the zip as `THIRD-PARTY-NOTICES.txt`; mention it in README-ALPHA.txt's Licences paragraph. Check that it contains the "Rust standard library" component and the mingw-w64 / gdtoa text.
- The small mingw-w64 files (item 10) are complete otherwise: `COPYING` (ZPL-2.1), `COPYING.MinGW-w64.txt`, `COPYING.MinGW-w64-runtime.txt` (contains the Lucent/David M. Gay gdtoa permission notice, the NetBSD notice and the others), `DISCLAIMER`, `DISCLAIMER.PD`; LLVM's `LICENSE.TXT` for libunwind. Missing only `AUTHORS` (item 5).

### 8. Nightly Rust with `-Z build-std`

The standard library is the same source and the same licences on nightly as on stable: MIT OR Apache-2.0, with the third-party parts that `library/` lists (fdlibm/Sun, NetBSD, compiler-rt under Apache-2.0 WITH LLVM-exception, `memchr` Unlicense OR MIT, `miniz_oxide`, `adler2`, `hashbrown`, `gimli`, `rustc-demangle`, `windows-sys`). `-Z build-std` compiles those sources from the `rust-src` component into our program; it does not add a licence. Notice duty = item 2. We do not distribute the nightly compiler, so nothing of it has to be offered. Nothing GPL/LGPL/AGPL (checked in the linker map earlier). OK as is.

### 9. Names: "win7", "Windows 7"

Nominative use (naming the product your software runs on, in plain text, with no logo and no suggestion of endorsement) is fine under EU and US law and within Microsoft's own trademark guidelines, which allow descriptive reference such as "for Windows". File name `…-win7-x86-alpha1.zip`, the folder name and the release title "RapidR 2.121.0 for Windows 7, 8 and 8.1 (32-bit), alpha 1" are fine. Keep: no Windows logo or flag, no "Windows 7 Edition" as a product name, "Windows 7" never inside the program's name or domain. `LEGAL.md` (in the zip) already carries the Microsoft trademark sentence; README-ALPHA.txt is what people read, so put the sentence there too (item 4). Mark the GitHub release "pre-release" (then it is never "latest", so the normal download page and any update check keep pointing at the normal release).

### 4. README-ALPHA.txt wording (add, English and Spanish)

- After the alpha paragraph: "This test build is provided as is, without warranty of any kind (see LICENSE). Use it at your own risk and keep a copy of anything you care about." (The MIT text says so, but this readme is what people read.)
- In "What it is": "Windows 7 and 8 no longer receive security updates from Microsoft; use this build on such a PC at your own risk, and preferably offline for anything sensitive." This is true, helps users, and removes any impression that we promise safety on an unsupported system. (Windows 8.1 is also past support, since January 2023.)
- Licences paragraph: "Windows is a trademark of the Microsoft group of companies. RapidR is not affiliated with or endorsed by Microsoft."
- Support line: today it reads "you can support it (voluntary, with nothing in return)". Use the agreed wording from `review-2026-10-22.md` 7c: "Support RapidR (Ruta Internet SRL)", still "voluntary, with nothing in return" (so a donation never reads as a purchase or as a promise of support or of fixes). The sentence "No part of this download is GPL or LGPL" is true by the audit; keep it, and keep the audit in the repository.
- Already good: "ALPHA TEST BUILD", "do not trust it with work you cannot lose", "NOT signed" with the safe way past the warning (never switch protection off), "compatible with RapidQ" only, no "clone".

### 3. The Windows 7 test VM

Facts. An unactivated Windows 7 runs in a built-in 30-day trial; Microsoft's licence terms still require a licence for use, and Windows 7 media is no longer sold. An ISO from archive.org is a copy somebody else uploaded: Microsoft did not authorise that copy, and an unofficial ISO may have been altered. Nothing from the VM is distributed (we only run our own program in it), so RapidR's own licensing is untouched; the exposure is Robert's/Ruta's own use of an unlicensed Windows copy for company work. Real-world enforcement against a few short test runs is very unlikely, but it is not "zero risk", and it is the kind of thing that costs credibility if it ever came up next to a public project. The old-pcs plan already says the same (no grey-market key, no VM licence).
- Do now: finish the current test round and then **delete the VM and the ISO**. Do not use `slmgr /rearm`, activators, or keys from key shops. While it exists: no network (or host-only), no personal data in it, no company credentials. If it is kept for a few days more, treat the 30-day trial as the hard limit.
- Check that the ISO matches the SHA-1 of Microsoft's original image if a reference list is at hand; if it does not match, delete it at once.
- Keep testing legitimately: (a) community testers on real PCs, as planned (Evgeny on 8.1 32-bit and the RapidQ group) with the `results.txt` the alpha already produces; (b) later, a cheap used PC that still carries its **original Windows 7 COA sticker** (genuine licence, real old hardware, and it also covers the CPU-only-drawing question that a VM cannot); (c) Windows 8.1 can be tried from Microsoft's own evaluation downloads if they are still offered (I did not verify availability), but it does not exercise the Windows 7 fallbacks of item 1, so it is not a substitute for (a) and (b).
- Wording: the alpha says "It has run on Windows 7 (32-bit, no updates) in a virtual machine". That is true and fine to keep; do not say which image or where it came from.

### Q2. Launch video music (Apple Loops)

What the licence says. The installed Logic Pro 12.4 ships "Logic Pro Software License Agreement", English part, document EA1924 (08/20/2024). I could not fetch Apple's web copy for the Mac app (404 on the guessed URLs), so this is the copy on Robert's own machine; it is the iPad-titled text of the same family, and the sample-content clause is the one Apple uses in the Mac versions too. Its Section 2.C ("Sample Content", which includes "audio loops") says, in substance:
- the Sample Content is Apple's and its licensors'; "all Sample Content included in the Apple Software may be used on a royalty-free basis to create your own original soundtracks for your audio and video projects";
- "You may broadcast, publish, display, and/or distribute your own soundtracks that were created using the Sample Content";
- but "individual Sample Content assets may not be commercially or otherwise distributed on a standalone basis, nor may they be repackaged in whole or in part as audio samples, clipart, music beds, sound effects, sound files, sound libraries, stock animation, or similar assets";
- new: you "may not use the Sample Content for training, testing, or evaluating any machine learning or artificial intelligence models, tools, algorithms, sound generators, or other software programs".

Reading for our case.
- A 2-minute product video with a layered, looped, faded soundtrack under it is exactly "your own original soundtrack for your video project", published on YouTube and GitHub. Apple's loops are made to be layered; choosing four loops from one pack, their entry points, loops and fades is the arrangement. The text does not ask for a minimum amount of transformation. The prohibition is about passing the *loops* on (as files, packs, "music beds" for others), not about audio that is part of a finished video. Company use is fine when the Mac and the Logic licence are Ruta's or Robert's with the company as the "commercial enterprise" (Section 2.A); if the licence was bought on a personal Apple ID and the video is posted by the company, keep the video on Robert's own channel or accept this as a Low risk.
- **Verdict: OK as is** for the video, with these conditions:
  1. Never publish the soundtrack on its own: no audio-only file, no mp3/wav of the mix, no music-service upload, no "music bed" download, not even as a bonus. The mix stays in the video only.
  2. Do not release the video under an open licence (CC-BY, MIT, public domain) and do not commit the mp4 or the mix to the git repository (the repo is MIT: anyone could lift the music). Attach the mp4 as a release asset or upload it, and write under it: "Music: Apple Loops (Apple Inc.), used under the Logic Pro licence; not covered by RapidR's MIT licence."
  3. Do not feed the raw loops or the mix to AI tools for analysis, testing or training (new AI clause); ffmpeg is not an AI model, so the mixing was fine.
  4. Keep proof: the list of the four file names and pack ("07 Chillwave"), the ffmpeg command, and a copy of the licence PDF, in `dist/2.121.0/launch/` next to the video (not in git).
- More margin if wanted (optional, not required): add one short part of our own, played with Logic's own instruments from a MIDI file (RapidR's `make_song.rr` can write one), so the track is no longer only Apple loops. That is better art direction and slightly stronger as "original", but it is not needed for the licence.

YouTube Content ID. Loop libraries are a known source of false claims: some other uploader's track that uses the same Apple loop gets registered in Content ID, and the claim then lands on every video that uses that loop. It is a claim, not a strike, and it is disputable.
- Upload as **unlisted** first, wait for YouTube Studio's copyright check to finish (it shows "Copyright: no issues" or lists the claim), then make it public.
- If a claim appears: dispute it in YouTube Studio ("Content" → "Copyright" → "Select action" → "Dispute") with the reason "licensed: Apple Loops from Apple Logic Pro, royalty-free for soundtracks under Section 2.C of the Logic Pro licence", naming the loops. Disputes are decided by the claimant, so be polite and factual; do not file a counter-notification (that one carries legal statements and exposes the account to a strike process) unless the dispute is rejected and the claim blocks the video; then ask for advice first.
- A claim that only turns on ads (no block) can be left alone; we run no ads.
- Same for the other platforms: no registration of the music with any distributor.
- If a claim ever becomes a nuisance, the fallback is a re-mix with our own MIDI part and fewer loops, or a different source; nothing needs changing now.

### Open items for the lane that fixes these

1. Replace the delay-load helper, rebuild, re-test, re-audit, update the docs (item 1).
2. Add `THIRD-PARTY-NOTICES.txt` (item 2) and `AUTHORS` (item 5) to `tools/win7/alpha.sh`'s list.
3. Edit README-ALPHA.txt (item 4), then rebuild the zip and its `.sha256`, and re-run `tools/win7/av_check.ps1`.
4. Delete the VM after the last test round (item 3).
5. Post the video as unlisted first, with the music line under it (Q2).

## Status of the addendum (lane WIN7-ALPHA-2, 2026-10-23 night)

| # | Item | Status | How |
|---|---|---|---|
| 1 | Replace the delay-load helper | **Done** | The simpler way of the addendum (3): no delay-loading at all. `crates/rapidr-cli/src/win7.rs` defines, for each of the six functions Windows 7 lacks (`RoGetActivationFactory`, `RoOriginateErrorW`, `PathCchStripPrefix`, `CoIncrementMTAUsage`, `CoCreateFreeThreadedMarshaler`, `CoTaskMemFree`), a small function of RapidR's own and the `__imp_<Function>` pointer the callers load their target from; the linker uses the object's own definition, so the import library's entry and the DLL never get in. On the first call the function finds the real one (`GetModuleHandleW` / `LoadLibraryW` from Windows' own folder / `GetProcAddress`) or answers "not available". `build.rs` no longer passes `--delayload`. Written from the public API documentation only; `delayimp.c`, `delay-f.c`, `delay-n.c`, `delayhlp.cpp` were never opened (record: `tools/win7/clean-room.md`, one row in `docs/legal/clean-room.md`). Checks: `tools/win7/link_audit.sh` 103 objects, the three helper objects gone, **0 flagged**; `imports.py rapidr.exe --against <the .exe that ran on Windows 7 RTM>`: no delay-loaded section, no `combase.dll` or `api-ms-win-core-*` anywhere, the only new import is `ole32.dll`'s nine plain functions (they were delay-loaded before; all in Windows 7 RTM); `rapidr.exe __win7_check` on Windows 11 finds all six natively, and a throwaway build pointed at a DLL that does not exist ran all six stand-ins without a crash. (The first version's stand-ins for `PathCchStripPrefix` and `CoIncrementMTAUsage` took no arguments although the 32-bit functions pop theirs: harmless while they were never called, wrong if they had been; the new ones have the real signatures.) **Not yet run on Windows 7 itself**: that is Robert's final run (`tests\run-tests.bat` now has a step 1b that prints where each of the six was found). |
| 2 | `THIRD-PARTY-NOTICES.txt` | **Done** | `tools/win7/alpha.sh` runs `rapidr notices tools-windows` (the Windows set of RapidR's programs, 297 components and 175 licence texts: all crates, Rust's standard library, mingw-w64 / gdtoa, libunwind, fonts) and puts the file in the zip; it then checks every crate of the i686-win7 graph against it and adds a three-line entry for `windows_i686_gnu` (windows-rs, same text as its siblings), the one crate the tool's x86_64/aarch64 sets do not have; any other missing crate stops the script. README-ALPHA.txt's Licences paragraph names the file. |
| 3 | Windows 7 test VM | **Open: Robert** | Not touched by this lane. Finish the final run, then delete the VM and the ISO (addendum 3). |
| 4 | README-ALPHA.txt wording | **Done** | English and Spanish: "as is, without warranty" sentence; Windows 7, 8 and 8.1 no longer receive security updates; the Microsoft trademark line; "Support RapidR" with the agreed wording copied from `README.md` (voluntary, received by Ruta Internet S.R.L., nothing in return, monthly renews until cancelled, payment handled by GitHub or Buy Me a Coffee). |
| 5 | mingw-w64 `AUTHORS` | **Done** | `tools/win7/licenses/mingw-w64/AUTHORS` (2,398 bytes, from the pinned commit `0d6dfbc3`), listed in `licenses\README.txt`; the zip has it. |
| 6, 7 | Music, Content ID | Not this lane | Conditions in the addendum stand. |
| 8, 9, 10 | OK as is | Unchanged | Item 9's trademark line is in README-ALPHA.txt now (item 4). |

New zip: `RapidR-2.121.0-win7-x86-alpha1.zip`, sha256 `7230e69ccc0d5599c931075d16bc134fe78a6ca3f36283b06849e64e8988121f` (27,135,833 bytes, 151 files). Antivirus check again (`tools/win7/av_check.ps1`, Windows 11 ARM VM, Bitdefender real-time on, 21:48): sha256 matched, all 151 files unzipped, version information intact, `rapidr.exe version`, `__win7_check`, `hello.rr` and `language.rr` exit code 0, Bitdefender's quarantine and `pending_actions.json` unchanged, folder removed.

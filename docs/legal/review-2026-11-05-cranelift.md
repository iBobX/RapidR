# Legal review, 2026-11-05: Cranelift (the JIT's code generator) as a dependency

This is the "lawyer agent" review (standing rule since 2026-10-22) for a new dependency family. It is a careful risk review with sources, **not legal advice** (US/EU practice; Uruguayan law not checked).

- **What is being added:** `cranelift-codegen`, `-frontend`, `-jit`, `-module`, `-native` (Bytecode Alliance, built in the wasmtime repository) and what they pull in, for the JIT described in `docs/jit-design.md` (next speed lane). Per `docs/build-for-every-platform-plan.md` §4 the **runners that end up inside users' exported apps carry the JIT** (x64 and ARM64), so it reaches users' commercial, closed-source programs, not only Studio.
- **Checked:** the exact dependency tree resolved in a scratch project (`/Volumes/RapidRBuild/legal-cranelift/probe`, `cargo metadata`, `cargo tree` for six native targets and wasm32); the real files in `~/.cargo/registry/src/*/` (LICENSE files, NOTICE files, source headers, `Cargo.toml` feature tables); `docs/jit-design.md`, `docs/build-for-every-platform-plan.md`, `deny.toml`, `crates/rapidr-cli/src/notices.rs`, `tools/third_party_notices.py`, `tools/release/home.py`, `LEGAL.md`, `docs/licensing.md`; the upstream repositories through `gh api` (regalloc2 issue 7 and pull request 15, the Bytecode Alliance governance repository); Apple's and Microsoft's own documentation; and a **real test on this Mac** (Apple silicon, macOS 27.2) of a tiny Cranelift JIT under four code-signing configurations (section 7).
- **Run here:** with RapidR's own `deny.toml` against the scratch tree: `cargo deny --offline check licenses` ok, `check bans` ok, `check sources` ok, `check advisories` ok (cached advisory database of 2026-10-30; not refreshed online).
- **Not run:** nothing was added to the repository, so `tools/third_party_notices.py`, `rapidr notices <kind>` and `tools/check_notices.py` were not run on the real tree (item C3 below).
- **Not read:** the Bytecode Alliance IP Policy (a PDF on its site); trademark registers (TMview, EUIPO, WIPO, USPTO); Apple's documentation on iOS JIT; Mac App Store review behaviour for `allow-jit`; hardened-runtime behaviour on Intel Macs (tested on Apple silicon only); Windows S mode / WDAC / antivirus behaviour; Linux SELinux `execmem` policy.

## Verdict: OK WITH CONDITIONS

Every crate in the shipped tree is declared Apache-2.0 WITH LLVM-exception, MIT, MIT OR Apache-2.0, Zlib or BSD-2-Clause, and RapidR's own `cargo deny` policy passes. **One real exception to that: a file inside `regalloc2` (a required dependency of Cranelift) still carries a Mozilla Public License 2.0 header.** RapidR's standing rule is no MPL in anything compiled into a program (it has removed or banned `symphonia`, `dwrote`, `option-ext` and `font-kit` for exactly that), and `cargo deny` cannot see a file header. That is condition C1 and it is a gate: the work can proceed on a branch, but the dependency must not be merged into `development` or reach a release until C1 is closed. The other conditions are mechanical. Two statements in `docs/jit-design.md` are wrong or unverified and must be corrected (section 9 below), and the macOS plan needs a custom memory provider (tested, section 7).

## Verdict table

| # | Item | Risk | Decision |
|---|---|---|---|
| 1a | Declared licence of every shipped crate (42 crate versions over six native targets; 34 on every one) | None | All on the `deny.toml` allowlist (section 1). |
| 1b | `regalloc2` 0.15.2, `src/lib.rs` carries an MPL-2.0 header | **Medium, gate** | **C1.** Upstream relicensed `src/ion/` in 2021 with Mozilla's sign-off but never touched `lib.rs` (section 2). |
| 1c | Build-time-only crates (4 code generators, `heck`) | None | Not compiled into any program. |
| 2a | Apache-2.0 WITH LLVM-exception duties for RapidR and for users' apps | None (notices already planned) | Binary: nothing required, listed anyway. Source form (the SDK's `vendor/`): licence copy and notices needed (section 3). |
| 2b | NOTICE files (Apache-2.0 section 4(d)) | None | No crate in the tree ships one, and the wasmtime repository has none. |
| 2c | Four shipped crates package no licence file | Low | The notices generator already prints the standard text for that case; verify (C3). |
| 3 | Patents | Low | Apache-2.0 section 3 grant, ends for whoever sues over patents in the work. No third-party patent search possible. |
| 4 | Export control | None | No cryptography compiled by default (section 5). |
| 5 | Trademarks "Cranelift", "Wasmtime", "Bytecode Alliance" | Low | Names only in the notices file, manifests and engineering docs; never in UI, Help, manual, README, posts (section 6). |
| 6a | macOS Hardened Runtime | **Product constraint, tested** | The stock `cranelift-jit` memory provider is **killed** even with `allow-jit`; a custom MAP_JIT provider works (section 7). |
| 6b | iOS / iPadOS | Product constraint | Interpreter only (as the design says). |
| 6c | Microsoft Store | Low | General-use capability for UWP; full-trust desktop apps unaffected. |
| 7 | Web runtime | None if kept out | Cranelift must never enter the wasm graph (C4). |

## 1. The exact dependency tree

Resolved on 2026-11-05 in a scratch project with the five crates pinned to `=0.137.1`, the newest **stable** release on crates.io (`0.138.0-rc.1` also exists as a release candidate; I resolved it too: **the same crate set**, only versions differ, so nothing below changes for it). Rust 1.97 is the minimum for 0.137 (the local toolchain is 1.98.1).

Shipped (normal) dependencies, `cargo tree -e normal`, per target: Linux x64 and ARM64 34 crates, macOS x64 35, macOS ARM64 36, Windows x64 and ARM64 39 (`windows-*` crates and `mach2` are the platform-specific ones). 42 crate versions in all. "New" means not in RapidR's `Cargo.lock` today (18 new crates; 9 more only at a different version; 15 already there).

| Crate | Version | Declared licence | Licence file in the package | New to RapidR | Note |
|---|---|---|---|---|---|
| cranelift-codegen | 0.137.1 | Apache-2.0 WITH LLVM-exception | yes | new | the compiler |
| cranelift-frontend, -jit, -module, -native, -entity, -bforest, -control, -codegen-shared | 0.137.1 | Apache-2.0 WITH LLVM-exception | yes (each) | new | |
| cranelift-assembler-x64 | 0.137.1 | Apache-2.0 WITH LLVM-exception | **no** | new | compiled on every target |
| cranelift-bitset | 0.137.1 | Apache-2.0 WITH LLVM-exception | **no** | new | |
| regalloc2 | 0.15.2 | Apache-2.0 WITH LLVM-exception | yes | new | **MPL header in `src/lib.rs`: section 2** |
| target-lexicon | 0.13.5 | Apache-2.0 WITH LLVM-exception | yes | new | |
| wasmtime-internal-core | 50.0.1 | Apache-2.0 WITH LLVM-exception | **no** | new | renamed from what the design doc lists |
| wasmtime-internal-jit-icache-coherence | 50.0.1 | Apache-2.0 WITH LLVM-exception | **no** | new | instruction-cache flush |
| gimli | 0.33.0 | MIT OR Apache-2.0 | yes | new | unwind tables (the `unwind` feature, which `cranelift-jit`, `-module`, `-native` and `-frontend` each switch on) |
| region | 3.0.2 | MIT (Copyright 2016 Elliott Linder) | yes | new | page protection |
| arbitrary | 1.5.0 | MIT OR Apache-2.0 | yes | new | **in the tree**: `cranelift-codegen`'s `std` feature turns on `cranelift-control/fuzz`; the design doc says "not enabled" |
| memmap2 | 0.9.12 | MIT OR Apache-2.0 | yes | version | RUSTSEC-2026-0186 is patched from 0.9.11 |
| mach2 | 0.4.3, 0.6.0 | BSD-2-Clause OR MIT OR Apache-2.0 | yes | version | macOS only |
| anyhow, bumpalo, allocator-api2, equivalent, fnv, foldhash, hashbrown (0.16.1 and 0.17.1), indexmap, libc, libm, log, rustc-hash, smallvec, bitflags 1.3.2 | | MIT OR Apache-2.0, MIT, Zlib (foldhash) | yes | present or version | support |
| windows-sys (0.52.0, 0.61.2), windows-link, windows-targets and the per-arch `windows_*_msvc` | | MIT OR Apache-2.0 | yes | present | Windows only |

Build-time only, never in a program: `cranelift-codegen-meta`, `cranelift-assembler-x64-meta`, `cranelift-isle`, `cranelift-srcgen`, `heck`. `serde`, `serde_derive`, `syn`, `quote`, `proc-macro2` appear in the lockfile only through the optional `enable-serde` feature and are **not** compiled.

- **Not in the allowed list:** nothing by declaration. Old-style `Apache-2.0 / MIT` (`fnv`) and `MIT/Apache-2.0` (`bitflags` 1.3.2) are already in RapidR's lock and pass.
- **GPL / LGPL / AGPL anywhere:** none (grep of every shipped Cranelift-family source file and every licence file).
- **MPL:** one file, section 2.
- **Optional features that would add licences, all OFF by default and to stay off:** `disas` (capstone), `incremental-cache` (`sha2`, `postcard`), `souper-harvest`, `enable-serde`, `pulley`; `cranelift-jit`'s `wasmtime-unwinder`. Default features are what the lane needs: trimming them does not change the crate set (tested with `default-features = false`).
- **Provenance sweep** (grep of every shipped Cranelift-family crate for "derived from / ported from / copied from / adapted from / MPL / GPL / copyright"): no third-party copyright line and no other derived-code statement. The only hits are in `regalloc2`: the MPL header, and "parts derived from regalloc.rs" (`checker.rs`, `domtree.rs`), which is Apache-2.0 WITH LLVM-exception as well.

## 2. regalloc2: the MPL header in `src/lib.rs` (condition C1)

What I found in the published crate `regalloc2-0.15.2` (and upstream `main`, commit `4a233442f7`, 2026-10-06):

- The crate says it is Apache-2.0 WITH LLVM-exception: `Cargo.toml` `license`, `README.md`, and 7 of its 32 source files carry the Apache-2.0 / LLVM-exception header. The README also says the crate began as, and is still about half, a port of IonMonkey's backtracking register allocator to Rust (IonMonkey is Mozilla's, MPL-2.0).
- The eight files in `src/ion/` that mention MPL say they were originally under MPL-2.0 and were subsequently relicensed to Apache-2.0 WITH LLVM-exception (pointing at upstream issue 7).
- **`src/lib.rs` (1,897 lines, about 18 % of the crate, the whole public API: `PReg`, `VReg`, `Operand`, `Allocation`, the `Function` trait, `MachineEnv`, `Output`, `run`) opens with a header that says a separate licence applies to that file and then uses the standard MPL-2.0 notice (the file is "subject to the terms of" MPL 2.0, with the usual pointer to mozilla.org/MPL/2.0).** The header says the file derives many details (register and constraint definitions, for example) from IonMonkey's `BacktrackingAllocator.h/.cpp` and `LIR.h`.
- **History (read through `gh api`):** the relicensing is pull request 15 (merged 2021-11-18, commit `cf0d515709`), with sign-offs from Mozilla (the GitHub user `ekr`, named in the PR as able to sign for Mozilla), Andy Wingo (for his changes to the original Firefox code) and Amanieu (three later contributions). Its description says it removes the MPL-2.0 licence from `src/ion/` and marks all files as covered by Apache-2.0 WITH LLVM-exception. The commit changed only `Cargo.toml`, `README.md` and `src/ion/*` (and deleted `src/ion/LICENSE`, the MPL text). **`src/lib.rs` was not changed in it, and has not been since.** The Bytecode Alliance's list of licence exceptions (governance repository) does not list regalloc2, and its default is Apache-2.0 WITH LLVM-exception for every hosted project.
- **Reading:** most likely a leftover header (the intent and the sign-offs point to "all files"). But I cannot show that from the sources: the file says MPL, the crate says Apache, and the one commit that relicensed did not touch it. `cargo deny` and `rapidr notices` read only the declared licence, so both would pass silently.

**What MPL-2.0 would ask if that file really is MPL** (file-level copyleft, section 3 of the MPL): keep the notices in that file; when giving the program to someone in executable form, also make that file's source available and tell recipients how to get it (a link to the exact crate version satisfies "reasonable means"); the rest of the program, and users' own code, stays under any terms (section 3.3, "Larger Work"), nothing else needs publishing. RapidR does not modify the file. So the worst reading is light, but it is a duty that every user shipping the runner would inherit, and it breaks RapidR's no-MPL rule (`docs/licensing.md` §4 and §7, `deny.toml`, `notices.rs`).

**Options, in the order I recommend:**
1. **Ask upstream, then pin the answer.** Open an issue (or comment on issue 7) at `bytecodealliance/regalloc2`: "src/lib.rs still has the MPL-2.0 header; was it covered by the November 2021 relicensing (PR 15)? If yes, please fix the header." A maintainer reply (or a new regalloc2 release with the header corrected) closes C1; record the link in this file and pin Cranelift to a release that uses that regalloc2. This is a public post: per the posting rules, **Robert decides** (account, wording). I did not post anything.
2. **Accept a narrow MPL-2.0 exception in writing** (Robert's decision, since it reverses a standing rule): the notices generator prints regalloc2's `src/lib.rs` as MPL-2.0 with the MPL text, the exact crate version and its crates.io / GitHub source link; `docs/licensing.md` §4 and §7 and `LEGAL.md` say so; a check that regalloc2 stays unmodified and that no other file in the tree gains an MPL header (`tools/check_notices.py` or a new guard that greps the registry sources for "Mozilla Public License", like `tools/check_archive_features.py` does for features).
3. **Do not ship Cranelift.** Keep the fast interpreter (already 2.5-6 times faster than 2.122). There is no practical way to use Cranelift without `regalloc2`, and rewriting `lib.rs` is not an option (the whole crate is built on its types).

## 3. What the licences require

**Cranelift's licence (read from the LICENSE file of each crate; identical in 11 crates, lines 205-219 are the exception).** Apache-2.0 plus the "LLVM Exceptions". The exception, in plain words and exactly in scope: *if, as a result of your compiling your source code, portions of this Software are embedded into an Object form of such source code, you may redistribute those embedded portions in that Object form without complying with Apache-2.0 sections 4(a) (give recipients a copy of the licence), 4(b) (mark modified files) and 4(d) (carry the NOTICE file's attributions).* A second paragraph lets you drop the patent (section 3) or indemnity (section 9) terms **only** if a court finds they conflict with GPLv2 when you combine this with GPLv2 software (irrelevant to RapidR: nothing GPL is linked).

What it does **not** waive: section 4(c) (keep copyright, patent, trademark and attribution notices in the **Source** form), section 3 (the patent grant and its termination), section 6 (trademarks), the disclaimer and liability limits. "Object form" is Apache-2.0's own term for any compiled form of source.

**Correction to `docs/jit-design.md` §9.** It says the exception "removes Apache-2.0's attribution requirement for code *compiled* by it: programs RapidR's JIT runs carry no notice". That is the wrong way round. The exception concerns **Cranelift's own code embedded in the binary that links it**. The machine code Cranelift **generates** for a user's BASIC program is not Cranelift's code (it contains no part of Cranelift; it is generated from the user's IR, lives in memory and is never distributed). Apache-2.0 asserts no rights over it and there is nothing to attribute either way.

**What each party owes:**

| Who ships what | Duty | Done by |
|---|---|---|
| RapidR's binaries (Studio, `rapidr`, runners) | Strictly nothing (exception, object form). Policy: list anyway ("listed anyway" in `docs/licensing.md` §4). I would **not** rely on the exception alone: it was written for LLVM's runtime pieces, and whether a whole statically linked library counts as "portions ... embedded" has not been tested in court; the notices cost nothing. | `rapidr notices` (generated per kind) |
| A user's exported app (runner + program) | Same: nothing by the exception, notices by RapidR's rule ("ship `THIRD-PARTY-NOTICES.txt` with your program", `LEGAL.md` "What to ship"). Users **do not** have to publish source, show Cranelift's name in the UI, or ask anyone. No advertising clause. | the notices file, as for every other dependency |
| The SDK's `vendor/` folder (`tools/release/home.py`: `cargo vendor` of the runtime crates' dependencies) | **Source form**: Apache-2.0 4(a) and 4(c) apply (the exception covers object form only). `cargo vendor` keeps each crate's licence files, **but four shipped crates have none** (cranelift-assembler-x64, cranelift-bitset, wasmtime-internal-core, wasmtime-internal-jit-icache-coherence), and the SDK is a source distribution. Cranelift enters `vendor/` only if the JIT is in the graph of `rapidr-runtime-core` or `-web`; per C4 it should not be, but if it is, the standard Apache text must travel with the SDK (the SDK's notices file does that). Also the regalloc2 header (section 2) is then in source form and travels intact. | C3, C4 |
| Anyone modifying Cranelift | 4(b): mark changed files; MPL for `regalloc2/src/lib.rs`. RapidR modifies nothing: **never `[patch]` or vendor-and-edit Cranelift.** | rule (C7) |

**NOTICE files (4(d)):** none in any of the 18 shipped crates, and the wasmtime repository root has `LICENSE` only. Nothing to carry.

**`THIRD_PARTY_NOTICES.md` and the per-program notices.** The notices generator (`crates/rapidr-cli/src/notices.rs`) already allows "Apache-2.0 WITH LLVM-exception", has its text in `crates/rapidr-cli/licenses/`, classifies a licence file containing "LLVM Exceptions" correctly, and for a crate that ships no licence file prints the standard text with the authors (or "the X developers (url)") as copyright holders (the "ships no licence file" branch, around line 697). Two new facts: the existing LLVM-exception crates in `THIRD_PARTY_NOTICES.md` are all the dual form `... OR MIT` (for which the generator picks MIT); **Cranelift's crates are LLVM-exception only, so this is the first time the notices carry that text for a crate with no alternative**, and the file grows by a new group and about 18 libraries (677 now). `cargo deny` accepts it today (verified).

**MIT and Apache-2.0 together:** compatible; RapidR stays MIT, the binary carries both sets of notices.

## 4. Patents

- **Grant:** Apache-2.0 section 3: every contributor (the Bytecode Alliance members and individuals who contributed code) grants a worldwide, royalty-free, irrevocable patent licence for the claims their contribution necessarily infringes. The exception does not change this.
- **Termination:** if a licensee sues anyone claiming that Cranelift (or a contribution in it) infringes a patent, that licensee's patent licences for Cranelift end. Relevant only to someone suing over Cranelift itself: not to Ruta Internet S.R.L. or to users in any realistic case. `docs/licensing.md` §4 already says it.
- **Third-party patents:** no patent search is possible here and none was made. Register allocation, SSA lowering and x64/ARM64 encoding are standard techniques; Cranelift is used in Firefox, wasmtime and several large products; the RustSec database (cached 2026-10-30) has one Cranelift advisory (RUSTSEC-2021-0067, a 2021 miscompilation, patched in 0.73.1) and no patent matter. Risk Low. `regalloc2`'s ancestry (Mozilla's allocator) added MPL's own patent grant for that code, again no known claim.
- **Practical security note, not legal:** a JIT is a sandbox-escape risk class (a miscompilation bug in the code generator becomes arbitrary code). Keep Cranelift current, keep `cargo deny check advisories` in `tools/regress.sh legal`, and keep the bytecode verifier (design doc §10 item 1) as the JIT's precondition.

## 5. Export control

No cryptography is compiled by default. `sha2` appears only under `cranelift-codegen`'s `incremental-cache` feature (`incremental_cache.rs`), which is off and stays off (C2); a grep for AES-NI / SHA instruction mnemonics in the x64 assembler crates found none; the only other "decrypt" hit is a comment about an ARM pointer-authentication instruction. A code generator is not an encryption item. **No change** to the position in `LEGAL.md` "Export" (public source, 15 CFR 734.3(b)(3), no filing), and `docs/licensing.md` §2.5's cryptography table needs no new row.

## 6. Trademarks and names

- Apache-2.0 section 6 grants no right to the names "Cranelift", "Wasmtime" or "Bytecode Alliance" beyond describing where the code comes from and reproducing NOTICE content. I did not search the registers and did not read the Bytecode Alliance IP Policy, so I do not know who owns which mark; the conservative rule needs neither.
- **Rule:** the names appear only in (a) `THIRD_PARTY_NOTICES.md` and the generated `THIRD-PARTY-NOTICES.txt`, (b) `Cargo.toml`, `Cargo.lock` and `use` lines (crate identifiers cannot be renamed), (c) engineering and legal documents (`docs/jit-design.md`, `docs/legal/`), where they are factual identifiers of a dependency. **Never** in the UI, Studio, Help, manual, README, ROADMAP, CHANGELOG, release notes, examples or posts: there it is "RapidR's JIT" or "the faster mode". No logos, no "powered by", no suggestion of affiliation. Nothing needs adding to `LEGAL.md` Trademarks while these names stay out of user-visible text (add the usual "independent of" sentence there only if that ever changes).
- Decision for Robert: `docs/` is public (the repository is public). Naming a dependency in an engineering document is ordinary factual use, but his memory rule "no other products in code, docs, UI or plans" was written about competing products, not dependencies; this review treats dependencies as the exception. Say if you want the names out of `docs/jit-design.md` too.

## 7. A JIT on app stores and hardened systems: product constraints

A JIT needs the operating system to let the process turn freshly written memory into code. That is exactly what stores and hardened modes restrict. This is a **product constraint**, not a licence problem.

### macOS (tested on this Mac: Apple silicon, macOS 27.2, ad-hoc signatures, a tiny Cranelift function called once)

Apple's own documentation (read 2026-11-05) says: `com.apple.security.cs.allow-jit` lets an app create writable-and-executable memory by passing `MAP_JIT` to `mmap`; the Hardened Runtime disallows this by default; without the entitlement, JIT-based frameworks may fall back to an interpreter and other JIT code may crash or misbehave. `allow-unsigned-executable-memory` is the broader exception, and Apple warns that it exposes the app to the usual memory-safety vulnerabilities.

The stock `cranelift-jit` memory provider (`SystemMemoryProvider`) does **not** use `MAP_JIT` or `pthread_jit_write_protect_np` (grep of its source; it maps anonymous memory read-write with `memmap2`, then `region::protect` to read-execute). The design doc §7 assumed it does. Result of the test:

| Memory provider | No Hardened Runtime | Hardened Runtime, no entitlement | Hardened Runtime + `allow-jit` | Hardened Runtime + `allow-unsigned-executable-memory` |
|---|---|---|---|---|
| Stock `cranelift-jit` | works | **process killed (SIGKILL, exit 137) when it first runs the generated code** | **process killed (exit 137)** | works |
| Custom provider: `mmap(... MAP_JIT)` + `pthread_jit_write_protect_np` + `sys_icache_invalidate` (about 25 lines against the public `JITMemoryProvider` trait) | works | `mmap` fails with EINVAL: a **normal, catchable error** | works | works |

Consequences:
1. **The JIT's macOS plan needs the custom MAP_JIT provider.** With the stock one, the only entitlement that works is the broad `allow-unsigned-executable-memory`: do **not** use it as a shortcut (it weakens every program the runner builds).
2. **"Try it and fall back" does not work with the stock provider** (it kills the process after the protection call "succeeded"). With the MAP_JIT provider the failure is a clean error at start-up: probe first (one `mmap(MAP_JIT)` of a page), and on failure run the interpreter, saying so in `rapidr run --verbose`.
3. **Exported macOS apps:** the build plan (§3) says they are signed ad hoc by RapidR's own small signer, with no certificate. That signer **does not exist in the code yet** (I searched `crates/` for a code-directory writer and its flag values: nothing), so I could not check what flags it will set. The rule for when it is written: it must **not** set the Hardened Runtime flag (`CS_RUNTIME`, 0x10000), which is what my "No Hardened Runtime" column corresponds to (an ad-hoc signature without it), or every exported app would die on its first JIT. A user who later signs and notarizes the app with the Hardened Runtime must add `com.apple.security.cs.allow-jit`; without it the app runs the interpreter (and with the stock provider it would crash, so the custom one is a release requirement). The manual says so in one paragraph.
4. RapidR's own Studio and runtime, signed as Ruta under the code-signing plan, get `allow-jit` and nothing else.
5. Untested: Intel Macs (a "Universal" app also runs there), and the Mac App Store (not planned; Developer ID distribution).

### iOS / iPadOS (RapidR's later mobile plans)
Interpreter only, as the design doc says (I did not verify the OS-level rule from Apple documentation in this review). Separately, App Store Review Guideline 2.5.2 (read today) bars apps from downloading, installing or executing code that introduces or changes the app's features, with a narrow educational exception where the source is viewable. That is about the **interpreter running user programs**, not about the JIT, and belongs in the mobile plan's own legal review.

### Microsoft Store and Windows
Microsoft's capability list (read today) has a **general-use** capability `codeGeneration` for UWP-style apps (it unlocks `VirtualProtectFromApp` and the `*FromApp` mapping functions); packaged full-trust desktop apps declare the restricted `runFullTrust` capability (Store approval) and use ordinary memory APIs. RapidR's exports are plain `.exe` files, not Store packages, so nothing applies today; anyone packaging a RapidR export for the Store follows the Store's rules. Not verified: Windows S mode, WDAC policies, Xbox, antivirus behaviour on code that makes memory executable.

### Linux
Not tested. Some SELinux profiles refuse `mprotect(PROT_EXEC)` on anonymous memory. The same fall-back (interpreter, with a message) must cover it, and must be an error, not a kill; test on the Ubuntu VM before release.

### 32-bit Windows (old-PC builds)
No x86-32 backend in Cranelift: interpreter only, as the plans say.

## 8. The six traps (CLAUDE.md), for this change

A compiler library adds no sign-up, personal data, analytics, email, payment or upload. **None of the six applies.** (Third-party assets: Cranelift is compiled in at build time, no server is contacted. Dependencies: all permissive, one MPL-headed file in section 2, nothing copied without a licence.)

## 9. Corrections for `docs/jit-design.md` (to apply when the lane starts)

- §9: the exception's effect is on Cranelift's code embedded in the binary, not on "code compiled by it" (section 3 above).
- §9 table: `wasmtime-jit-icache-coherence` is now `wasmtime-internal-jit-icache-coherence` 50.0.1; add `wasmtime-internal-core`, `cranelift-assembler-x64`, `cranelift-bitset`, `memmap2`, `mach2`, `libm`, `foldhash`, `fnv`, `bitflags` 1.3.2, `equivalent`, `indexmap`, `anyhow`; build-time `cranelift-assembler-x64-meta`, `cranelift-srcgen`; `arbitrary` is **enabled** (through `std`), `serde` is not.
- §9: say that `regalloc2` carries an MPL-headed file and that C1 decides it; and that "No GPL / LGPL / AGPL" is true of the declared licences and of every file read.
- §7: "maps it with `MAP_JIT`, writing through `pthread_jit_write_protect_np`" is true **only with a custom provider**; the stock provider is killed under the Hardened Runtime even with `allow-jit` (tested, section 7). Add the probe-then-fall-back rule.
- §7: Cranelift version pin and the "features that stay off" list (C2).

## Required (C1 to C8)

- [ ] **C1 (gate): regalloc2's `src/lib.rs` MPL header.** Close it by option 1 (upstream confirms or fixes; pin the fixed release; link the answer here) or option 2 (Robert approves a narrow, documented MPL exception with the generator, docs and a guard). No merge of the dependency into `development`, and no release containing it, until then. Development on a branch is fine.
- [ ] **C2: pins and features.** Pin all five crates to one release (`=0.137.1` or the fixed one), commit `Cargo.lock`; keep `disas`, `incremental-cache`, `souper-harvest`, `enable-serde`, `pulley` and `wasmtime-unwinder` off; add a guard like `tools/check_archive_features.py` (in `tools/regress.sh legal`) that fails if one is turned on or if the shipped graph gains a crate or licence not in section 1 or any "Mozilla Public License" file header outside the approved one.
- [ ] **C3: notices.** Run `python3 tools/third_party_notices.py`, `rapidr notices` for macos, windows-x86_64, windows-aarch64, linux-x86_64, linux-aarch64 and `web`, and `tools/check_notices.py`: confirm the 18 new crates appear, the four with no licence file get the standard Apache-2.0 WITH LLVM-exception text with a copyright line, and (per C1) regalloc2's entry is right. Re-run `cargo deny check` (all four checks, advisories online) on the real `Cargo.lock`: RapidR's resolution may pick other patch versions than the scratch tree.
- [ ] **C4: placement.** `jit` is a non-default Cargo feature of the interpreter host used by the runner stub, Studio and the CLI; it is **not** in `rapidr-runtime-core` (native builds never embed the VM) or anything the web runtime builds, and never on iOS or 32-bit x86. Test: `cargo tree -p rapidr-vm-host-web` and `-p rapidr-runtime-web` show no `cranelift*`, `regalloc2` or `wasmtime-internal-*`; `rapidr notices web` lists none. If the JIT does end up in the SDK's vendored graph, the SDK ships the notices file beside `vendor/` (section 3).
- [ ] **C5: macOS.** Custom `JITMemoryProvider` (MAP_JIT, `pthread_jit_write_protect_np`, icache invalidate); start-up probe with fall-back to the interpreter and a message; never the stock provider and never `allow-unsigned-executable-memory`; RapidR's ad-hoc export signer (not written yet) must not set the Hardened Runtime flag (test: `codesign -d --verbose=2` on an export shows no `runtime`); manual paragraph on `allow-jit` for users who notarize; a real test of a hardened, `allow-jit`-signed runner. Same fall-back test on Linux (SELinux) before release.
- [ ] **C6: wording.** Apply section 9's corrections to `docs/jit-design.md`; no Cranelift / wasmtime / Bytecode Alliance in any user-visible text (section 6).
- [ ] **C7: never modify.** No `[patch]`, no vendored-and-edited copy of any crate in the tree.
- [ ] **C8: docs.** `docs/licensing.md` §2 (the desktop kinds' crate lists and counts), §4's LLVM-exception row (add: "in source form, as in the SDK's vendored crates, 4(a) and 4(c) still apply") and §7 (if C1 option 2); `LICENSES.md` only if a native library or MPL exception is added.

Recommended, not blocking: a short note in `docs/licensing.md` §5 ("Machine-code compiler: Cranelift, Apache-2.0 WITH LLVM-exception; `regalloc2` derived from an allocator first published under the MPL, relicensed with Mozilla's sign-off").

## Decisions for Robert
1. **C1:** may I draft the upstream question for `bytecodealliance/regalloc2` (to post as the helper account, or you), or would you rather accept the narrow MPL exception, or skip Cranelift and keep the fast interpreter? My recommendation: ask upstream (free, probably a one-line fix), pin the answer, and meanwhile build the lane on a branch.
2. **Names in `docs/jit-design.md`** (public repository): keep as engineering text (recommended) or remove.
3. **Mobile and stores:** confirm there is no plan for the Mac App Store or iOS JIT; both stay interpreter-only / out of scope.

## Sources (read 2026-11-05)
- Resolved tree: `/Volumes/RapidRBuild/legal-cranelift/` (`probe/Cargo.toml`, `probe/Cargo.lock`, `meta-0.137.1.json`, `normal-<target>.txt`, `signtest/`); crate files: `~/.cargo/registry/src/index.crates.io-*/{cranelift-codegen,cranelift-assembler-x64,cranelift-bitset,cranelift-control,cranelift-jit,regalloc2,region,target-lexicon,wasmtime-internal-core,wasmtime-internal-jit-icache-coherence,gimli,libm,...}` (LICENSE files, `Cargo.toml`, source headers, `src/memory/system.rs`, `region/src/os/unix.rs`).
- regalloc2: https://github.com/bytecodealliance/regalloc2 (issue 7 "Relicense fully under Apache-2.0 WITH LLVM-exception"; pull request 15, commit `cf0d515709`; `src/lib.rs` on `main`, commit `4a233442f7`).
- Bytecode Alliance governance: https://github.com/bytecodealliance/governance (`PROJECT_LICENSE`; `projects/license-exceptions/README.md`, which does not list regalloc2).
- wasmtime repository root (no NOTICE): https://github.com/bytecodealliance/wasmtime
- Apple: Allow Execution of JIT-compiled Code entitlement (`com.apple.security.cs.allow-jit`) and Allow Unsigned Executable Memory entitlement, developer.apple.com documentation (read as the documentation's JSON); App Store Review Guidelines 2.5.2, https://developer.apple.com/app-store/review/guidelines/
- Microsoft: App capability declarations (the `codeGeneration` row; restricted capabilities), https://learn.microsoft.com/windows/uwp/packaging/app-capability-declarations (source in MicrosoftDocs/windows-dev-docs)
- RustSec advisory database (local copy, 2026-10-30): RUSTSEC-2021-0067 (cranelift-codegen), RUSTSEC-2026-0186 (memmap2).
- RapidR: `docs/jit-design.md`, `docs/build-for-every-platform-plan.md`, `docs/licensing.md`, `LEGAL.md`, `deny.toml`, `crates/rapidr-cli/src/notices.rs`, `tools/release/home.py`, `tools/check_notices.py`; earlier reviews `docs/legal/review-2026-10-29-rarchive.md` (format and the "superset notices" reasoning) and `docs/legal/review-2026-10-22.md`.

# Legal review, 2026-10-29: RArchive (lane RZIP, `crates/rapidr-archive`)

This is the "lawyer agent" review (standing rule since 2026-10-22) for new dependencies, new file formats and a new component name. It is a careful risk review with sources, **not legal advice** (US/EU practice; Uruguayan law not checked).

- **What is being built:** `RArchive`, a component that opens zip, 7z, tar, tar.gz, .gz, .xz and .zst files (and makes zip, tar, tar.gz, .gz), with zip-slip and zip-bomb guards. RAR is recognised by its signature and refused. The engine is linked into users' (possibly commercial) programs: interpreted, native and web/wasm.
- **Checked:** the real files in `~/.cargo/registry/src/*/<crate>-<version>/` (LICENSE files, README, CHANGELOG, source headers), `Cargo.lock` and `cargo tree -p rapidr-archive` in the worktree (commit `57d2e33f`), `THIRD_PARTY_NOTICES.md`, `deny.toml`, `docs/licensing.md`, `LICENSES.md`, the manual page, and the upstream repositories through `gh api` and web pages.
- **Run here:** `cargo deny --offline check licenses` passes, `bans` and `sources` pass, `tools/third_party_notices.py --check` passes ("up to date"). `cargo deny check advisories` fails on `RUSTSEC-2026-0414` in `pulldown-cmark`; that is not in this lane's tree and not a licence matter (flagged for the next dependency pass).
- **Not run:** `rapidr notices <kind>` (no built `rapidr` binary in this worktree), so the per-program notices file was not read; see item 5.
- **Not read:** the IETF IPR page for RFC 8878; the 7z format text file inside the 7-Zip source (it is not in the registry crates); trademark registers (TMview, EUIPO, WIPO, USPTO TSDR).

## Verdict: MERGE AFTER FIXES

Nothing in the shipped code is copyleft, and nothing was copied from 7-Zip's LGPL code or from unRAR. The fixes are documentation and notices (the "no cryptography in the web runtime" claim is now wrong), plus one build check.

## Verdict table

| # | Item | Risk | Decision |
|---|---|---|---|
| 1a | Licence of every shipped crate (files, not only Cargo.toml) | None | All permissive and on the `deny.toml` allowlist; table in §1. |
| 1b | Apache-2.0 §4(d) NOTICE obligations | None | None of the Apache-2.0 crates ships a NOTICE file. §4(a) (a copy of the licence) is met by the notices generator. |
| 2a | `lzma-rust2` provenance | None | Port of XZ for Java (public domain: Lasse Collin, Igor Pavlov, Brett Okken) through the earlier `lzma-rust` crate. No 7-Zip LGPL code found. |
| 2b | `sevenz-rust2` provenance | None | Own Rust implementation of the 7z container by the `sevenz-rust` author, with LZMA/BCJ code from the crate above. No 7-Zip LGPL code found. BCJ2 decoder follows the LZMA SDK's `Bcj2.c` (public domain). |
| 2c | `ruzstd` provenance | None (as built) | Written from the Zstandard format document (RFC 8878), MIT. One file translates a function from facebook/zstd, but it is in the `dict_builder` feature, which is off. |
| 3a | ZIP format, patents (Deflate) | Low | Read/write of stored and Deflate entries is allowed by the APPNOTE's own terms. Never implement strong encryption or patching (PKWARE licence needed). |
| 3b | LZMA, LZMA2, BCJ, Delta, Zstandard (patents) | Low | No patent known; the authors themselves give no guarantee. Read-only here. Keep as is. |
| 3c | 7z format | Low | No restriction found on implementing it. The 7-Zip name and logo are not used. |
| 3d | RAR: never read | None | **Decision confirmed.** Nothing from unRAR; the 32-byte fixture is the signature plus zeros. Wording fix recommended (§3). |
| 4a | Name `RArchive` | Low | Generic word; no register hit or product found. Allowed. |
| 4b | Format names `zip`, `7z`, `RAR`, `gzip`, `xz`, `zstd` | Low | Allowed as plain descriptive words in running text. Do not use `zip64`, `Deflate64`, `PKZIP`, `7-Zip` logo or name as features (§4). |
| 5 | `THIRD_PARTY_NOTICES.md` | None | Every shipped crate is listed with its licence group. Licence texts come from the generated per-program file; one build check required. |
| 6 | bzip2 later: licence `bzip2-1.0.6` | Low, acceptable | Permissive BSD-style; allowed later after three small changes (§6). |
| 7 | Cryptography statements in the docs | Medium (wrong claim) | **Fix before merge:** `docs/licensing.md` §2.5 and `LICENSES.md` say the web has none and list only MySQL/keyring. SHA-256 (`sha2` 0.11) and ZipCrypto code are now in. |

## 1. The licence of each shipped crate (read from the files)

The tree for `rapidr-archive` with the lane's features (from `cargo tree`):

| Crate | Version | Declared | File(s) in the package | NOTICE file | Note |
|---|---|---|---|---|---|
| `zip` | 9.0.5 | MIT | `LICENSE` | no | Reviewed 2026-10-22 (`review-2026-10-22-zip9.md`). Its AES code (`aes-crypto`) is a feature that is **off**. |
| `flate2` | 1.1.9 | MIT OR Apache-2.0 | `LICENSE-MIT`, `LICENSE-APACHE` | no | Used with `zlib-rs`. |
| `zlib-rs` | 0.6.8 | Zlib | `LICENSE` | no | |
| `crc32fast` | 1.5.0 | MIT OR Apache-2.0 | both | no | |
| `indexmap` (+ `equivalent`, `hashbrown`) | 2.13.0 | Apache-2.0 OR MIT | both | no | |
| `memchr` | 2.8.0 | Unlicense OR MIT | `UNLICENSE`, `LICENSE-MIT`, `COPYING` | no | |
| `typed-path` | 0.12.4 | MIT OR Apache-2.0 | both | no | |
| `nix` | 0.31.3 | MIT | `LICENSE` | no | macOS/BSD only. |
| `miniz_oxide`, `adler2`, `simd-adler32` | | MIT/Zlib/Apache | | no | Through `flate2`; already in the notices. |
| **`lzma-rust2`** | 0.23.0 | Apache-2.0 | `LICENSE` (standard Apache text, appendix placeholder `[yyyy] [name of copyright owner]` not filled in) | **no** | See §2. |
| **`sevenz-rust2`** | 0.24.1 | Apache-2.0 | `LICENSE` (same) | **no** | See §2. Only `deflate` is enabled; its defaults (`aes256`, `bzip2`, `ppmd`, `compress`) are **off**. |
| **`ruzstd`** | 0.9.1 | MIT | `LICENSE` (© 2019 Moritz Borcherding) | no | `dict_builder` and `hash` features off. |
| **`sha2`** | 0.11.1 | MIT OR Apache-2.0 | `LICENSE-MIT`, `LICENSE-APACHE` | no | Used by `lzma-rust2`'s `xz` feature for the XZ SHA-256 integrity check. |
| **`digest`, `block-buffer`, `crypto-common`, `cpufeatures`** | 0.11.3, 0.12.1, 0.2.2, 0.3.1 | MIT OR Apache-2.0 | both | no | RustCrypto. |
| **`const-oid`** | 0.10.2 | Apache-2.0 OR MIT | both | no | RustCrypto. |
| `hybrid-array`, `typenum` | | | | | Already in the tree before; listed in the notices. |
| `js-sys`, `wasm-bindgen` (+ proc-macro crates) | | MIT OR Apache-2.0 | | | Pulled by `sevenz-rust2` for wasm32 only; already in the tree. |

- **Result:** all permissive, all on the `deny.toml` allowlist. `cargo deny check licenses` passes.
- **Apache-2.0 §4:** the obligation is to give recipients a copy of the licence and keep any NOTICE content. The three Apache-only crates (`lzma-rust2`, `sevenz-rust2`) ship no NOTICE file, so the only duty is the licence text; the per-program `THIRD-PARTY-NOTICES.txt` already carries Apache-2.0 in full.
- **The licence files do not name a copyright holder** (the Apache appendix placeholder is unfilled and `Cargo.toml` has no `authors`). Nothing to fix on our side: the licence text is the licence. The notices generator lists the crate by name, version and upstream link (`github.com/hasenbanck/...`).
- **Dual-licensed crates:** the notices use the MIT option (it has the copyright lines); that is what the generator already does for the rest of the tree.
- **Build scripts / C code:** `lzma-rust2`, `sevenz-rust2` and `ruzstd` have no build script and no C. `lzma-rust2`'s `optimization` feature (turned on by `sevenz-rust2`) has hand-written assembly and `unsafe` in `range_dec.rs`; that is not a licence matter, but it is code that reads untrusted input (a note for the security audit).

## 2. Provenance: is any LGPL 7-Zip code inside?

**Short answer: no.** 7-Zip's main code is LGPL-2.1 (parts with the unRAR restriction; BSD-3-Clause for LZFSE/Zstandard; BSD-2-Clause for XXH64), per https://www.7-zip.org/license.txt. The **LZMA SDK** is "placed in the public domain" (https://www.7-zip.org/sdk.html). Everything below comes from the public-domain side.

**`lzma-rust2` 0.23.0** (repo `hasenbanck/lzma-rust2`, Apache-2.0, first commit 2025-06-28)
- The README and `src/lib.rs` say it is "ported from tukaani xz for java" and is "a fork of the original, unmaintained lzma-rust crate" (itself the LZMA part of the earlier `sevenz-rust` project, split out on 2023-01-31 per that CHANGELOG).
- XZ for Java is public domain: its `COPYING` says all files "have been written by Lasse Collin, Igor Pavlov, and/or Brett Okken" and "have been put into the public domain" (Android and Bazel mirrors of the file, and Debian's copyright file with the tag `public-domain`). A public-domain source can be relicensed as Apache-2.0 by whoever adds to it; the crate's own additions are Apache-2.0.
- The only mention of 7-Zip in the source is a comment on the pre-computed price table in `enc/range_enc.rs` ("for which 7zip has a function"). A table of logarithm values is a fact, the encoder is not compiled in our build (`encoder` feature off), and the SDK function it echoes is public domain anyway.
- BCJ filters (x86, ARM, PPC, SPARC, IA64, RISC-V) are the xz-java / LZMA SDK filters (both public domain). The BCJ2 decoder (`filter/bcj2/decode.rs`) uses the field and constant names of the LZMA SDK's `Bcj2.c` (`BCJ2_NUM_STREAMS`, `probs[2+256]`, `range`, `code`, `ip`, `temp`), i.e. it is a port of the SDK's C code. The SDK files are marked public domain. No attribution is required; a courtesy line is cheap (§5).
- No file header, README or CHANGELOG line mentions LGPL, GPL or 7-Zip source.

**`sevenz-rust2` 0.24.1** (repo `hasenbanck/sevenz-rust`, Apache-2.0)
- "A fork of the original, unmaintained sevenz-rust crate" (history back to 2022-09; original by `dyz1990`, whose old repository no longer answers on GitHub, so the earliest history could not be read). The CHANGELOG shows the work as feature additions of that author and contributors (bcj, delta, AES, bzip2, zstd, BCJ2), not as imports of 7-Zip source.
- The container code (`reader.rs`, `writer.rs`) follows the 7z format description (names such as `K_START_POS` are the format's property IDs; names of format constants are facts). No copied C++ or comments from 7-Zip's `7zIn.cpp`/`7zOut.cpp` were found; comments cite 7-Zip only for behaviour ("7-Zip's own encoder never exceeds this").
- The LZMA/LZMA2/BCJ/Delta decoders are the `lzma-rust2` code above.
- **Not compiled in, and each would need its own review before being turned on:** `ppmd` (`ppmd-rust`, likely a port of 7-Zip's `Ppmd7.c`), `bzip2` (§6), `zstd` (`zstd-sys`, C), `brotli`, `lz4`, `aes256` (also bans `aes`/`cbc` in `deny.toml`, so it would fail the build).

**`ruzstd` 0.9.1** (repo `KillingSpark/zstd-rs`, MIT)
- "A pure Rust implementation of the Zstandard compression format, as defined in RFC8878." Source comments cite the format document in facebook/zstd (`doc/zstd_compression_format.md`) and the RFC; there is no libzstd C code, and no copied code.
- One file, `dictionary/cover.rs`, says "A translation of `COVER_computeEpochs()` from facebook/zstd" and used Facebook's implementation as a reference. zstd is BSD-3-Clause OR GPL-2.0 (the user chooses BSD), so even there the obligation would only be a BSD notice. It sits behind the `dict_builder` feature, which is **off** in `Cargo.toml`, so it is not compiled into any program. **Rule: never enable `dict_builder`** without a new review (it would bring a BSD-3 notice and `fastrand`).

## 3. Format and patent issues

**ZIP (PKWARE APPNOTE 6.3.10, 2022-11-01).** The APPNOTE (§1.4.1) permits use "solely for the purpose of creating products, programs and processes that read and write files in the ZIP format". Two limits:
- Section 1.4.3 and 4.5.8.3 say strong encryption and patching (PKPatchMaker) are patented PKWARE technology needing a licence. RArchive refuses encrypted entries and never decrypts (tests with `encrypted.zip`), and implements no patching or Deflate64. **Rule: never add AES/strong-encryption, Deflate64, patching or Central Directory Encryption.** (Traditional ZipCrypto is the open, old scheme and is not the licensed technology; the code is still never used.)
- Section 1.3.1 lists `PKWARE`, `PKZIP`, `SecureZIP`, `PKSFX` as registered marks and `PKPatchMaker`, `Deflate64`, `ZIP64` as marks of PKWARE. The manual page says "zip64" once as a feature. Replace with "zip files over 4 GB" (§4).
- Deflate (RFC 1951): no patent issue; the algorithm is in zlib, whose FAQ says it is patent-free. Stored and Deflate are the only methods enabled.

**LZMA / LZMA2 / XZ / 7z filters.** The xz FAQ says the authors are "not aware of any patents" affecting LZMA/LZMA2 but cannot guarantee that no third party patent exists; Igor Pavlov published the SDK as public domain. Nothing more can be known without a patent search, which is out of scope under the rule "decide conservatively". Risk is Low, and we are the decoder side only (no encoder compiled in).

**Zstandard.** RFC 8878 is an Informational RFC edited by Yann Collet (Facebook/Meta); the reference code is BSD-3 OR GPL-2.0. The searches did not find a patent claim or a licence restriction; the IETF IPR disclosure page for RFC 8878 was **not read**. Risk Low. Read only.

**7z format.** The 7-Zip site describes the format as 7-Zip's; the format description has shipped in the source distribution's `doc` folder since 2015 and the LZMA SDK includes 7z decoding code (public domain). The text of that spec file and its own licence were not read. A file format is an interface (17 USC §102(b); the same reasoning as in `review-2026-10-29-mcp.md` §1) and no site says that implementing it needs permission. **Rule:** do not use the "7-Zip" name or logo, and do not copy 7-Zip's LGPL source or documentation text; say "7z files".

**RAR: the decision to never read it is right.**
- The unRAR source is licensed to "handle RAR archives" free of charge but "cannot be used to develop RAR (WinRAR) compatible archiver and to re-create RAR compression algorithm, which is proprietary", the paragraph must be reproduced in documentation and source comments, and Fedora and Debian list the licence as non-free and GPL-incompatible (not OSI approved). It cannot go into a program under our "permissive only" rule.
- The RAR format and its compression are proprietary (Alexander Roshal / RARLAB). A clean-room reader is lawful in principle, but the format is only partly documented and any mistake would sit in users' commercial programs. Refusing costs nothing.
- What the lane has: the 8-byte magic `Rar!\x1a\x07\x01\x00` (a fact), a refusal message, and a 32-byte fixture (signature plus zero bytes, checked with `xxd`). That is zero risk. Nothing from unRAR is in the tree.
- **Wording (recommended, not blocking):** the message says "the format isn't open". RAR's decompression source is public under a restricted licence, so "isn't open" can be argued with. Prefer "RAR is a closed, proprietary format" (message in `crates/rapidr-archive/src/lib.rs` `RAR_REFUSED`, `docs/manual/archives.md`, `docs/manual/reference/members.md`, and the expected-output files in `tests/conformance/cases/`).
- **Never add RAR reading** unless a later review finds a permissive, independently written implementation; do not accept a "unrar" crate or binding (the restriction would follow it into users' programs).

## 4. The name "RArchive" and format names

- **"RArchive":** a web search found no trademark, product or library with this name (the nearest hit is an unrelated 2002 GPL tool called `rarch`, "Removable Archive"). It is a generic word with RapidR's `R` prefix (like `RHttpServer`). Allowed. Not a register search (TMview, EUIPO, WIPO, USPTO TSDR were not searched); an optional free check by a person is worth five minutes before the first public post. If a clash ever shows, rename and keep the old name as a silent alias (the existing RapidR practice).
- **"zip", "tar", "gzip", "xz", "7z":** descriptive names of formats and file extensions. Fine in running text and as extensions. "ZIP" has no known exclusive claim as a bare format name; PKWARE's marks are the ones in §3.
- **"RAR":** a trademark of its owner (RARLAB / win.rar GmbH); used only to say which format is refused. Fine as a plain word. No WinRAR logo or name as a feature.
- **"7-Zip"**: a product name. Do not use it in UI, Help, manual or posts. (`docs/manual/archives.md` and the engine's rustdoc do not use it today; one test comment lists the command-line tool `7zz` that made the fixtures; that is an internal test note, and output of a tool is not covered by the tool's licence.)
- **"Zstandard", "zstd":** names used by Meta for the format and tool. Plain descriptive use is fine.
- **Attribution line** (add to `LEGAL.md` where the other third-party names are covered): "RAR is a trademark of its owner; 7-Zip, PKWARE, PKZIP, ZIP64, Deflate64 and Zstandard are names or marks of their owners. RapidR uses these names only to say which file formats it reads and writes. RapidR is independent of all of them."
- **Test fixtures** were made by `make_fixtures.py` with the system's zip, 7zz, bsdtar, gzip, xz and zstd on content written for the test (the made-up `hello.z80`). Output files of those tools are not covered by the tools' licences. Fine.

## 5. `THIRD_PARTY_NOTICES.md` in the worktree

- **Coverage:** the file is generated from `cargo metadata` and "is up to date" (`tools/third_party_notices.py --check`). It now lists 676 libraries; every crate in §1 appears: `lzma-rust2` and `sevenz-rust2` in "Apache-2.0", `ruzstd` in "MIT", `sha2` 0.11.1, `digest` 0.11.3, `block-buffer` 0.12.1, `crypto-common` 0.2.2, `const-oid` 0.10.2, `cpufeatures` 0.3.1 in "Apache-2.0 OR MIT", `zip`, `flate2`, `zlib-rs`, `crc32fast`, `indexmap`, `memchr`, `typed-path`, `nix` as before. **No crate is missing.**
- **Licence texts:** that file is by design a table (name, version, upstream); the texts and copyright lines are in the generated, per-program `THIRD-PARTY-NOTICES.txt` (`rapidr notices <kind>`, made from each crate's own licence files, per `docs/licensing.md`). Item to verify before merge: build `rapidr`, run `rapidr notices macos`, `rapidr notices web` (and one other desktop kind) and check that `lzma-rust2`, `sevenz-rust2` (Apache-2.0 text), `ruzstd` (© 2019 Moritz Borcherding), `sha2` and the RustCrypto crates (MIT text) appear. The generator works by *kind*, not per program, so a program that does not use RArchive still carries the notices (a superset, harmless). The kind's graph must contain `rapidr-archive`: the diff turns on `archive` in `rapidr-vm-host-native` (desktop runners) but I did not see where the web runtime does; if the web runtime includes it, `web` must list these crates too, and the licensing document's web table must be extended.
- **`LICENSES.md` §7** (native libraries and the crypto table, line ~257) and **`docs/licensing.md`** need the edits in §7 below.
- **Courtesy lines** (optional, free): in `docs/licensing.md` add "RArchive: zip, tar, gzip from open formats (PKWARE APPNOTE, RFC 1950-1952); xz and LZMA from XZ for Java (public domain) through `lzma-rust2`; 7z through `sevenz-rust2`; Zstandard from RFC 8878 through `ruzstd`; no 7-Zip or unRAR source."

## 6. bzip2 for a later phase: is `bzip2-1.0.6` acceptable?

**Yes.** SPDX `bzip2-1.0.6` is the original bzip2 licence (Julian Seward), a BSD-style licence with four conditions (text read from `libbz2-rs-sys` 0.2.6):
1. Source redistributions keep the copyright notice and disclaimer.
2. Do not misrepresent the origin; an acknowledgement in documentation "would be appreciated but is not required".
3. Altered **source** versions must be marked as altered.
4. The author's name may not be used to endorse the product without permission.

No copyleft, no patent clause, no fee, and no condition on binaries beyond the disclaimer. It allows commercial, closed-source use, which is our test (no copyleft; permissive). The generator will print the text anyway.

Three small steps when bzip2 is added (none today):
1. `deny.toml` `allow`: add `"bzip2-1.0.6"`; `crates/rapidr-cli/src/notices.rs` `ALLOWED` plus a licence text file in `crates/rapidr-cli/licenses/` (the unit test keeps the two lists equal, and the generator must detect the text: it currently detects MIT, Apache, BSD, ISC, Zlib, 0BSD and others, not this one); `tools/check_notices.py` and the licence table in `docs/licensing.md`.
2. Prefer the pure-Rust route. `libbz2-rs-sys` 0.2.6 (Trifecta Tech Foundation) is a **translation of C bzip2 into Rust** (its LICENSE keeps Seward, Mena Quintero and Snyder's copyright and adds Trifecta's), so its true licence is `bzip2-1.0.6`, which is fine. `bzip2` 0.6.1 (MIT OR Apache-2.0) uses it by default; its `bzip2-sys` feature builds the **C** library: do not turn that on. `bzip2-rs` 0.1.2 is a separate, safe pure-Rust *decoder* declared MIT/Apache-2.0 with no mention of being derived from the C code; for a read-only phase that is the simplest licence, but check its source headers once more at that time.
3. Keep the bzip2 reader read-only and bounded like the rest (MaxSize, MaxRatio); that is a security matter, not a licence one.

## 7. Cryptography statements that become wrong (fix before merge)

`docs/licensing.md` says "no cryptography, is in the web runtime" (about line 141), §2.5 lists the web as "nothing (the browser does HTTPS; `crc32fast` is a checksum)", and `LICENSES.md` §7 lists SHA-1/SHA-256 "not the web". With RArchive:
- **SHA-256** (`sha2` 0.11 with `digest`, `block-buffer`, `crypto-common`, `const-oid`, `cpufeatures`) is compiled in wherever `xz` is on, **including the web runtime if it carries the archive code**: it checks the integrity of .xz files. A hash hides nothing, and §2.5 already says so for SHA-1/SHA-256.
- **ZipCrypto** (`zip` crate, `zipcrypto.rs`) is compiled into the `zip` crate regardless of features (the old PKWARE stream cipher). The RArchive engine refuses encrypted entries before reading them, so it is not used; whether the linker drops it is not guaranteed, so list it honestly: "legacy ZipCrypto code exists in `zip`; RArchive never calls it".
- **Not compiled in (keep it so):** the AES features of `zip` (`aes-crypto`) and `sevenz-rust2` (`aes256`); `aes`, `cbc` and `hkdf` are banned in `deny.toml`, `notices.rs` and `tools/check_notices.py`, so enabling either feature would fail the check. Say so in §2.5.
- `LEGAL.md` "Export" ("only standard cryptography … listed under 'Encryption'") stays true if §2.5 is updated. No change to the export position: a hash and an unused legacy cipher in public source do not change the reading of 15 CFR 734.3(b)(3).

## The six traps (CLAUDE.md), for this feature

| Trap | Result |
|---|---|
| 1 Children | No accounts or personal data. Not applicable. |
| 2 Third-party assets | No network use, no fonts or scripts. Not applicable. |
| 3 Analytics and replay | None. |
| 4 Marketing email | Not applicable. |
| 5 Subscriptions | Not applicable. |
| 6 Uploads (DMCA) | RArchive opens files on the user's own computer (or the web page's own files); nothing is hosted or shared by RapidR. Not applicable. |

Dependencies (the CLAUDE.md rule): all permissive and allowlisted; no GPL/LGPL/AGPL; nothing copied from third parties without a licence (§2).

## Required before merge

- [x] **Docs, cryptography (§7):** update `docs/licensing.md` (the "no cryptography in the web runtime" sentence, §2.5 table rows for desktop and web, the "Not compiled in: zip `aes-crypto`, sevenz-rust2 `aes256`" note) and the crypto row in `LICENSES.md` §7 (SHA-256 for the XZ check; `sha2` 0.11 beside 0.10). *(Done, RZIP 2026-10-29: `docs/licensing.md` §2.3's sentence, §2.5's rows (SHA-256 and the unused ZipCrypto wherever RArchive is; AES not compiled in, banned, and guarded), `LICENSES.md` §7's table (`sha2` 0.10 and 0.11, ZipCrypto never called), and `LEGAL.md`'s Encryption paragraph, whose "a web build contains none of these" was wrong too.)*
- [x] **Per-program notices (§5):** build `rapidr` and run `rapidr notices` for the desktop kind and `web`; confirm `lzma-rust2`, `sevenz-rust2`, `ruzstd`, `sha2` 0.11 and the RustCrypto crates appear with their licence texts; if the web runtime includes `rapidr-archive`, extend the web table in `docs/licensing.md`. Run the CLI's notices unit tests. *(Done: `rapidr notices` for macos, windows-x86_64, linux-x86_64 and web list `lzma-rust2`, `sevenz-rust2` (Apache-2.0 text), `ruzstd` (MIT, © 2019 Moritz Borcherding), `sha2` 0.11, `digest`, `block-buffer`, `crypto-common`, `const-oid` and, on the desktop, `cpufeatures` (x86/ARM only: not on wasm) with their texts; the web runtime has RArchive, so `docs/licensing.md` §2.3's web table now has the counts `rapidr notices web` gives (176 crates) and RArchive's row; the CLI's notices unit tests and `tools/check_notices.py` pass.)*
- [x] **Names (§3, §4):** replace "zip64" in `docs/manual/archives.md` with "zip files over 4 GB" (and anywhere else it appears); add the attribution sentence to `LEGAL.md`; no "7-Zip", "WinRAR", "PKZIP" in UI, Help, manual or posts. *(Done: "zip files over 4 GB" in the manual and the engine's comment; the attribution in `LEGAL.md` › Trademarks; no 7-Zip, WinRAR or PKZIP in UI, Help, manual or examples — a test comment and `LICENSES.md` §6's heading said 7-Zip / PKZIP, now 7zz / Zip. The RAR message is now "RAR files can't be opened: it's a closed, proprietary format. Ask for a .zip or .7z.")*
- [x] **Feature guard (§2):** a comment (or a `cargo deny`/`check_notices.py` check) that `ruzstd` `dict_builder`, `sevenz-rust2` `ppmd`/`bzip2`/`zstd`/`brotli`/`lz4`/`aes256` and `zip` `aes-crypto` stay off without a new review. The `Cargo.toml` already says so for most; add `dict_builder`. *(Done: the list in `crates/rapidr-archive/Cargo.toml`, and `tools/check_archive_features.py` — from `cargo metadata`, desktop and wasm, default and all features — in `tools/regress.sh legal`; it was seen failing with `dict_builder` turned on.)*
- [x] **Courtesy line** in `docs/licensing.md` about the formats and sources (§5). Optional but cheap. *(Done: `docs/licensing.md` §5, "Archives (RArchive)".)*

Recommended (not blocking): change the RAR refusal wording to "closed, proprietary format" (§3), and re-run `cargo deny check` (advisories) after the pulldown-cmark update.

## Decisions for Robert
1. **None needed** to merge. The fixes above are mechanical.
2. **Optional, free:** a TMview / EUIPO / USPTO name search for "RArchive" before the first public post (five minutes). A web search found nothing.
3. **Later:** bzip2 reading is allowed (§6). RAR reading stays refused.

## Sources (read 2026-10-29)
- Crate files: `~/.cargo/registry/src/*/{lzma-rust2-0.23.0, sevenz-rust2-0.24.1, ruzstd-0.9.1, sha2-0.11.1, digest-0.11.3, block-buffer-0.12.1, crypto-common-0.2.2, const-oid-0.10.2, cpufeatures-0.3.1, zip-9.0.5, libbz2-rs-sys-0.2.6, bzip2-0.6.1, bzip2-rs-0.1.2}` (LICENSE files, README, CHANGELOG, `Cargo.toml`).
- Repositories (via `gh api`): https://github.com/hasenbanck/lzma-rust2 and https://github.com/hasenbanck/sevenz-rust (both Apache-2.0, "fork of the original, unmaintained" crates); https://github.com/KillingSpark/zstd-rs (MIT).
- XZ for Java is public domain (COPYING, as mirrored): https://bazel.googlesource.com/bazel/+/refs/tags/6.1.1/third_party/xz/LICENSE and https://sources.debian.org/src/xz-java/1.9-1/debian/copyright
- 7-Zip licence structure: https://www.7-zip.org/license.txt ; LZMA SDK public domain: https://www.7-zip.org/sdk.html ; 7z page: https://www.7-zip.org/7z.html
- xz/LZMA patent FAQ: https://mirrors.lzu.edu.cn/git/xz/src/branch/master/doc/faq.txt
- PKWARE APPNOTE 6.3.10: https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT (§1.3.1, §1.4.1-1.4.3, §4.5.8.3; the last 74,585 characters of the file were not read)
- Zstandard format: https://www.rfc-editor.org/info/rfc8878
- UnRAR licence: https://spdx.org/licenses/UnRAR.html , https://fedoraproject.org/wiki/Licensing/Unrar , Debian's copyright file for `unrar-nonfree`
- Name search: web search for "RArchive" (no hit); `rarch.sourceforge.net` (unrelated).
- Earlier reviews: `docs/legal/review-2026-10-22-zip9.md`, `docs/legal/review-2026-10-22.md`, `docs/legal/review-2026-10-29-mcp.md` (format-as-interface reasoning), `docs/legal/six-traps-preview4.md`.

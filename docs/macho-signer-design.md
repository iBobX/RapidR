# The ad-hoc Mach-O signer (design, 2026-11-07)

Plan B2 of [build-for-every-platform-plan.md](build-for-every-platform-plan.md)
(sections 3 and 5). An Apple-silicon Mac refuses to run an arm64 program that
has no valid code signature. Until 2.123.0, `rapidr build` signed each `.app`
by running Apple's `codesign --force --sign -`, so only a Mac could make a Mac
app that opens. This signer does the same job in pure Rust, in
`crates/rapidr-package/src/macho_sign.rs`:

- **bytes in, bytes out:** it runs no process and touches no file, so it
  compiles to wasm32 and works the same on macOS, Windows, Linux and in the
  web Studio;
- **ad hoc only:** a CodeDirectory of SHA-256 page hashes, an empty
  requirements set, an empty signature slot and the ad-hoc flag. There are no
  certificates, no identity and no Apple software. Developer ID signing and
  notarisation stay out of scope: they need the user's own Apple account, on
  a Mac;
- **the same bytes as `codesign`:** for the files RapidR makes, our output is
  byte for byte what `codesign --force --sign -` writes on macOS 27.2. The
  golden tests check this, so whatever macOS accepts from `codesign` it
  accepts from us.

## 1. Clean room

The legal decision of 2026-10-22 (7b) says this signer is written from public
format descriptions only. No other signer's code was read; several are
MPL, GPL or AGPL. Sources:

1. Apple's published headers in the macOS SDK (Xcode 27):
   `Kernel.framework/Headers/kern/cs_blobs.h` (the CodeDirectory, SuperBlob
   and BlobIndex layouts, the magics, slot numbers, hash types, `CS_ADHOC`,
   `CS_EXECSEG_MAIN_BINARY`, the version numbers),
   `usr/include/mach-o/loader.h` (`mach_header_64`, `segment_command_64`,
   `section_64`, `linkedit_data_command`, `LC_CODE_SIGNATURE`, `LC_UUID`)
   and `usr/include/mach-o/fat.h` (`fat_header`, `fat_arch`, `fat_arch_64`).
2. Apple Technical Note TN2206, *macOS Code Signing In Depth*: what an ad-hoc
   signature is, the bundle layout (`_CodeSignature/CodeResources`), and the
   resource rules (`rules`, `rules2`, `omit`, `optional`, `nested`, `weight`).
3. Apple Technical Note TN3126, *Inside Code Signing: Hashes*: the
   CodeDirectory, page hashes, special slots (Info.plist, requirements,
   resources) and the cdhash.
   (2 and 3 were known before this lane and not fetched again: the lane ran
   without network. Every detail taken from them was checked against
   `codesign`'s output, item 6.)
4. The `codesign(1)` manual page on this Mac (its options, `--identifier`).
5. FIPS 180-4, the Secure Hash Standard (SHA-1, which the `files`
   dictionary of CodeResources still uses), and RFC 4648 (base64).
6. **Black-box observation** of what `codesign --force --sign -` writes on
   macOS 27.2: small C programs and copies of `/usr/bin/true` signed with it,
   then dumped with our own Python script and `codesign -d -vvvvvv`, the
   output only. The layout rules in section 3 that the documents above don't
   state (the page size per CPU, the space reserved for the signature, the
   `__LINKEDIT` sizes, the universal re-layout, the exact CodeResources text)
   come from these observations.

Hashing uses the `sha2` crate, already in the tree (Cargo.lock; MIT or
Apache-2.0). SHA-1 and base64 are written here (a few dozen lines each, from
the standards), so the signer adds no crate.

## 2. What a signed Mach-O looks like

```text
mach_header_64
load commands ... LC_SEGMENT_64 __LINKEDIT ... LC_CODE_SIGNATURE {dataoff, datasize}
__TEXT ... __DATA ... __LINKEDIT [symbols, fixups ... | signature]
                                                        ^ dataoff (16-aligned)
signature = SuperBlob 0xfade0cc0 (big-endian), 3 entries:
  slot 0        CodeDirectory 0xfade0c02, version 0x20400
  slot 2        Requirements  0xfade0c01, empty (count 0, 12 bytes)
  slot 0x10000  BlobWrapper   0xfade0b01, empty (8 bytes): no CMS, ad hoc
CodeDirectory:
  header (88 bytes: flags = CS_ADHOC, hashType = SHA-256, hashSize = 32,
          platform 0, pageSize = log2 page, codeLimit = dataoff,
          execSeg = __TEXT's file range, CS_EXECSEG_MAIN_BINARY for an executable)
  identifier, NUL-terminated
  special slots -n .. -1   (-3 CodeResources, -2 requirements, -1 Info.plist)
  code slots 0 .. n-1      SHA-256 of each page of [0, codeLimit)
```

The page hashes cover the file from its first byte up to the signature,
including the load commands. So the header is patched first (the new
`LC_CODE_SIGNATURE`, the grown `__LINKEDIT`) and hashed after. Nothing
depends on the signature's own bytes, so nothing is circular.

## 3. The layout rules (as `codesign` does them)

| What | Rule |
|---|---|
| Page size | 16 KiB for arm64 and arm64e slices, 4 KiB for x86_64 (the brief said 4 KiB; `codesign` on macOS 27.2 uses 16 KiB on arm64, so we do too: both are valid, but only this gives the same hashes) |
| Where the code ends | at the old signature's `dataoff` when the file has one (linker-signed Rust and C programs do); else at the end of `__LINKEDIT` |
| Signature offset | the code's end rounded up to 16; the gap is zero |
| Signature space (`datasize`) | the SuperBlob's length + 8, rounded up to 16 |
| `__LINKEDIT` | `filesize` grows to the signature's end; `vmsize` = `filesize` rounded up to 16 KiB (it can shrink) |
| A new `LC_CODE_SIGNATURE` | added after the last load command (`ncmds` + 1, `sizeofcmds` + 16), only if there is room before the first section's data |
| Bytes after the SuperBlob | what the old file had there (re-signing in place), else zero; they are not hashed |
| CodeDirectory | version 0x20400, flags `CS_ADHOC` (a linker signature's `CS_LINKER_SIGNED` is dropped), identifier at offset 88, no team id, no scatter, `codeLimit64` 0 |
| Special slots | 2 for a plain program (-1 Info.plist is zero, unbound); 3 for an app's main executable (-1 = SHA-256 of Info.plist, -3 = SHA-256 of CodeResources) |
| Universal | each slice signed alone; slices kept in their order, each at a 2^14-aligned offset (`align` raised to 14), zeros between |
| Identifier | an app's `CFBundleIdentifier`; a plain program's is the caller's (`codesign` uses the file name without its extension, `-`, and hex of `"UUID"` + the arm64 slice's LC_UUID: `default_identifier`) |
| Hash kinds | SHA-256 only. `codesign` adds a SHA-1 CodeDirectory (slot 0, SHA-256 moved to slot 0x1000) when the program's minimum macOS is below 10.11.4; RapidR's programs need 10.13 (Intel) or 11.0 (Apple silicon), where SHA-256 alone is what `codesign` writes too |
| CodeDirectory version | 0x20400 always. `codesign` writes 0x20100 for a Mach-O with no LC_BUILD_VERSION at all (no linker makes one; only hand-made test files) |

## 4. The app's seal (CodeResources)

`_CodeSignature/CodeResources` is an XML plist with four dictionaries. It is
written with exactly the text `codesign` writes: tabs, keys sorted by UTF-16
code unit, `&`, `<` and `>` escaped, each hash as one base64 line in
`<data>`, and a newline at the end.

- `files` (version 1, for old systems): each file under `Resources/` with its
  SHA-1. A file under `Resources/<lang>.lproj/` is `optional`; one under
  `Resources/Base.lproj/` isn't; `Resources/<lang>.lproj/locversion.plist` is
  left out.
- `files2` (version 2, what macOS checks): each file under `Resources/` with
  its SHA-256 (`hash2`), with the same `optional` rule.
- `rules` and `rules2`: Apple's default rule sets, verbatim (TN2206). They
  tell a verifier which files to expect and how strictly.

Info.plist and PkgInfo are not in `files2` (`omit`): Info.plist is bound by
the main executable's special slot -1, and PkgInfo is left out by the rules.
The main executable is the signed code itself.

`sign_app` takes the whole bundle in memory and refuses what it can't seal
honestly: a file outside `Contents/Resources` other than Info.plist, PkgInfo
and the main executable (that would be nested code, which needs its own
signature first), paths with `..` and absolute paths. A `.DS_Store` file
stays in the app but out of the seal, as the rules say.

## 5. API (`rapidr_package::macho_sign`)

```rust
pub struct Bound<'a> { pub info_plist: Option<&'a [u8]>, pub code_resources: Option<&'a [u8]> }
pub fn sign(macho: &[u8], identifier: &str, bound: Bound) -> Result<Vec<u8>, String>;
pub fn sign_app(files: &mut Vec<(String, Vec<u8>)>) -> Result<String, String>; // paths in the .app; gives the program's
pub fn code_resources(resources: &[(&str, &[u8])]) -> Vec<u8>;               // paths in Contents/: "Resources/..."
pub fn default_identifier(file_name: &str, macho: &[u8]) -> String;
pub fn check(macho: &[u8]) -> Result<Vec<Signed>, String>;                   // reads a signature back, re-hashes it
```

Its one caller is the packer's call site (BUILD-CORE,
[build-core-design.md](build-core-design.md) §6):
`macos::sign_ad_hoc(files, app_root, info) -> Result<Signature, String>`
inside `pack`. It takes the app's files below `Name.app/` out of the build's
`Files` (moved, not copied: a runner is tens of megabytes), calls
`sign_app`, and puts them back with their executable marks: the signed
program and `Contents/_CodeSignature/CodeResources`. Refused (no program at
`Contents/MacOS/<exe>`, nested code, a damaged program): the files go back
as they were and the build stops with the reason. Every macOS app any build
makes passes there, folder builds and `--targets` archives alike, on every
system and in the browser, so a Windows or Linux build for macOS makes an app
that opens. The CLI's `codesign` bridge is deleted (one implementation), and
`Signature` has one kind, `AdHoc`.

`crates/rapidr-package/examples/adhoc_sign.rs` signs a program or a `.app`
folder from the shell, for checking by hand (it reads and writes files; the
signer doesn't).

## 6. Errors (refused, never half-signed)

A truncated or corrupted input is refused with a sentence that says what is
wrong and where, and nothing is changed. For example (the unit tests check
each): "not a Mach-O program (it starts with 4d5a9000: a Windows .exe?)",
"the Mach-O is cut short: its load commands end at byte 376, the file at
100", "segment __LINKEDIT runs past the end of the file (it ends at byte
17184, the file at 17174): cut short?", "load command 0 (at byte 32) is
broken: its size 0 doesn't fit", "no __LINKEDIT segment (an object file, not
a linked program?)", "15 bytes after the end of __LINKEDIT: a signed program
can carry nothing after its code (put data in the app's Contents/Resources)",
"a 32-bit Mach-O: macOS runs 64-bit programs only (arm64, x86_64)", "no room
for the signature's load command (...): link it with more header padding",
"the universal file's arm64 slice (bytes 16384..) isn't inside the file",
and for an app "Contents/MacOS/helper: an app RapidR signs keeps its files in
Contents/Resources ...". `check` finds a page changed after signing ("page 1
of 3 doesn't match its hash").

## 7. Tests

- Unit tests on synthetic Mach-O files that the tests build themselves: thin
  arm64, thin x86_64, universal, already signed, a dylib, and every kind of
  corruption above. They check the layout, recompute every page hash
  independently, and check the bundle seal text.
- Golden tests (macOS, `codesign` used only to compare): the same input signed
  by `codesign --force --sign - -i <id>` and by us must be byte-identical.
  Inputs: the synthetic files (arm64, x86_64, universal, a dylib, one signed
  before under another name), the test program itself (a linker-signed arm64
  Rust program), a copy of `/usr/bin/true` (universal x86_64 + arm64e +
  arm64e, Apple-signed) and its x86_64 slice, and a sample `.app` with
  resources, `.lproj` folders and odd file names (`&`, `<`, quotes, `é`).
- Signing again is a no-op: `sign(sign(x)) == sign(x)`.
- Acceptance on this Mac (by hand, `examples/adhoc_sign.rs` and `rapidr
  build`): the RapidR runner (thin arm64, and universal with an x86_64 build)
  and a built app signed by our signer only, then `codesign --verify --deep
  --strict -v`, `codesign -dv` ("Signature=adhoc"), the same bytes as
  `codesign`'s, and the app opened: as built, under Rosetta for the x86_64
  slice, and after a zip, a quarantine flag and an unzip.

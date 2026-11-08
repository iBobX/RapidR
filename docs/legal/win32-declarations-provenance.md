# Provenance: RWin32.inc, RapidR's Windows API declarations

What this records: where every line of `crates/rapidr-preprocessor/src/libraries/RWin32.inc`
comes from, so anyone can check it and make the same file again. The legal basis is
[win32-declarations-review.md](win32-declarations-review.md) (2026-11-07): option (a1), Microsoft's
own Win32 metadata as the windows-rs project publishes it, MIT OR Apache-2.0, used under MIT.

## The input (pinned)

| | |
|---|---|
| Crate | `windows-sys` **0.61.2**, from crates.io (the version already in RapidR's `Cargo.lock`) |
| crates.io SHA-256 of `windows-sys-0.61.2.crate` | `ae137229bcbd6cdf0f7b80a31df61766145077ddf49416a728b02cb3921ff3fc` (Cargo.lock's checksum; the file in cargo's cache has the same hash) |
| Built from | `microsoft/windows-rs` commit `32c3144490c016fe496a0aed769bce60987a2e9d`, path `crates/libs/sys` (the crate's `.cargo_vcs_info.json`) |
| Licence | `license = "MIT OR Apache-2.0"` (the crate's `Cargo.toml`); `license-mit` is the MIT License, "Copyright (c) Microsoft Corporation." |
| Per-file terms | none: the 249 `.rs` files have no licence or copyright lines (checked: the only comments are two doc lines in `src/core/literals.rs`) |
| Files read | `src/**/*.rs` (249 files) and `license-mit`: 250 files |
| SHA-256 of what was read | `743aef372edd33530b5ab6a60da0b0da567203cc8892bb55b9c9de42dac330fe` (for each file in path order: its path, a NUL byte, its length as 8 bytes little-endian, its bytes) |
| Network | none: the tool reads cargo's own cache (`~/.cargo/registry/src/*/windows-sys-0.61.2`) |

The tool refuses any other version, a crate not built from that commit, and a licence field that
isn't `MIT OR Apache-2.0` ("stop, the lawyer checks it again"): changing the pin is a decision,
reviewed again (review §Recommendation 7).

## The tool

`tools/gen-win32` (RapidR's own code, MIT): a line reader for the crate's generated Rust (functions
`windows_link::link!(…)`, `pub const`, `pub type`, `pub struct` / `pub union`), the C layout rules of
Windows' compiler (natural alignment, `packed(n)`, 8-byte pointers on 64-bit Windows and 4 on 32-bit),
and RapidR's own type mapping and syntax. `cargo run -p rapidr-gen-win32 --release` writes the file;
`-- --check` says whether the committed file is what the input gives.

Tests (`cargo test -p rapidr-gen-win32 --release`): two generations give the same bytes and the
committed file is them (and the `.gz` unpacks to it); every TYPE's size in RapidR (its packed fields and the gaps written out) is
the structure's size on 64-bit and on 32-bit Windows; 40 structures' 64-bit sizes agree with Rust's
own `size_of` of the same metadata (windows-sys compiled for this machine: the layout oracle).

## The output

| | |
|---|---|
| File | `crates/rapidr-preprocessor/src/libraries/RWin32.inc` |
| Size | 4,465,054 bytes, 120,093 lines; the copy the binaries carry, `RWin32.inc.gz` beside it (gzip, level 9, no time or name in its header): 788,021 bytes |
| SHA-256 | `fd199c3985a31de0fa815c2515151b077c269b93250d87e4764110b2881a49b6` (the `.gz`: `5b95c1d123ef6f212532e2936c38a53beb436864535298eda4f50d3c357415dc`) |
| Contents | 55,710 constants (2,916 of them text), 3,675 TYPEs (1,734 with a 64-bit and a 32-bit layout), 6,506 functions of 29 DLLs; classic names: 1,028 functions, 325 TYPEs |
| Header | the notice of the review's "Exact notice text" (Microsoft's copyright line, the MIT permission notice, the trademark line), the pinned input and its hashes |

Left out on purpose (counted by the tool): the 115 names RapidR's built-in RAPIDQ.INC already has
(RapidR's compatibility layer gives those, with the same numbers — the tool checks each value; the
file lists their names so the compiler can give them to a classic program without RAPIDQ.INC), GUIDs (1,063), floating-point and structure values,
text RapidR strings can't hold, functions with variable arguments or a structure of more than
8 bytes by value, items for one 64-bit kind of Windows only (x64 or ARM64), names that are RapidR
words or built-ins, and whole API areas a covered DLL only touches (Windows Search, cryptography,
Active Directory, HID usages, UI Automation …: `NAMESPACES_LEFT_OUT` in the tool).

## RapidR's own additions (not from the metadata)

Checked against Microsoft's documentation, names and facts only:

| Name | Is | Why | Documentation |
|---|---|---|---|
| `POINTAPI` | the same TYPE as `POINT` | the classic name for POINT, which older programs use | <https://learn.microsoft.com/en-us/windows/win32/api/windef/ns-windef-point> |
| `SIZEL` | the same TYPE as `SIZE` | the SDK's second name for SIZE | <https://learn.microsoft.com/en-us/windows/win32/api/windef/ns-windef-size> |
| `tagINPUT` | the TYPE for the INPUT structure | `INPUT` is a statement of RapidR's; `tagINPUT` is the structure's C tag name | <https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-input> |

The classic unsuffixed names (`MessageBox` = `MessageBoxA`, `LOGFONT` = `LOGFONTA`) are not
additions: they are worked out from the metadata (every A/W pair's ANSI one), as the SDK's own
ANSI build names them.

## What was not used

No Windows SDK header, no `.winmd` (the NuGet package's licence conflicts: review option a2), no
`WIN32API.TXT` or any RapidQ-era include, no Wine, MinGW, Free Pascal or JEDI file, nothing written
from memory. A third-party classic include (a Rapid-Q port of WIN32API.TXT, kept outside the
repository) was used only to **compare names** — which declaration, constant and TYPE names older
programs use — read by a script that keeps the names and nothing else; its text was never copied,
and the file is not in the repository or any package.

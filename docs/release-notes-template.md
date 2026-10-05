# RapidR @VERSION@

RapidR runs and builds RapidQ programs. It targets full compatibility with
RapidQ on all three of its runtimes — the **native compiler** (your program as
a native executable), the **interpreter** (the RapidR Runtime, and standalone
executables that carry it) and the **web** (the same program in a browser) —
and an existing RapidQ program behaves as it does under RapidQ.

RapidR also goes further than RapidQ, without changing what RapidQ programs
do: a data-science stack (data frames, arrays, charts), an AI stack, SQLite and
MySQL, high-DPI screens and accessibility, and more.

RapidR is not a clone of RapidQ or Delphi. It is an original implementation,
written from the ground up in pure Rust, that is compatible with RapidQ.

<!-- What's new in this release: from CHANGELOG.md -->
## What's new

- …

## Downloads

| | SDK: the IDE, the compiler, the runtime | Runtime only: runs programs |
|---|---|---|
| **Windows** (x64, ARM64) | `RapidR-@VERSION@-windows-<arch>-setup.exe` | `RapidR-Runtime-@VERSION@-windows-<arch>-setup.exe` |
| **macOS** (Apple silicon and Intel) | `RapidR-@VERSION@-macos-universal.dmg` | `RapidR-Runtime-@VERSION@-macos-universal.dmg` |
| **Linux** (x86_64, aarch64) | `rapidr-@VERSION@-linux-<arch>.tar.gz`, `rapidr_@VERSION@_<arch>.deb` | `rapidr-runtime-@VERSION@-linux-<arch>.tar.gz`, `rapidr-runtime_@VERSION@_<arch>.deb` |
| **Web** | `rapidr-web-@VERSION@.zip`: the web IDE, a folder for any static web host | |

The SDK includes the runtime. Each install is for you alone and needs no
administrator rights (the `.deb` packages install system-wide).

**Start working right away:** install, open the IDE (Windows: Start menu >
RapidR IDE; macOS: RapidR; Linux: RapidR IDE in your applications), write a
program and press Run. No other software is needed to run programs or to make
standalone executables (`rapidr build prog.bas --interp`). Native builds
(`rapidr build prog.bas`) compile your program with Rust: `rapidr setup`
installs it once, after asking.

**File types:** a `.rrbc` (a compiled RapidR program) runs on a double click;
a `.rr` or `.bas` opens in the IDE, and "Run" (Windows) or "Open With > RapidR
Runtime" (macOS, Linux) runs it. A program downloaded from the internet asks
once before it runs. On macOS and Linux a script that starts with
`#!/usr/bin/env rapidr` runs directly.

<!-- Keep or drop, per what the release is: -->
**Unsigned downloads:** this release is not code-signed. On macOS, the first
time, right-click the app and choose Open (or System Settings > Privacy &
Security > Open Anyway). On Windows, SmartScreen may say "Windows protected
your PC": choose More info > Run anyway.

**Verify a download:** `SHA256SUMS` lists every file's SHA-256
(`shasum -a 256 -c SHA256SUMS`, or `Get-FileHash` on Windows).
`rapidr-@VERSION@.cdx.json` is the release's software bill of materials
(CycloneDX).

## Licences

RapidR is MIT-licensed, and so is everything it ships, or under similarly
permissive open-source licences (Apache-2.0, BSD, ISC, zlib, Unicode, MPL-2.0
for unmodified font crates, OFL for the built-in fonts): nothing GPL, LGPL or
AGPL. The programs you build with RapidR are yours, to ship as open source or
commercially. Every package carries `LICENSE`, `LICENSES.md` and
`THIRD_PARTY_NOTICES.md`.

Built from commit @COMMIT@.

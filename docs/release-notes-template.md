# RapidR @VERSION@

RapidR is a modern BASIC with its own IDE, RapidR Studio. Write a program once
and run it three ways: as a **native** executable, as a standalone
**interpreted** app (no extra software needed), or on the **web**, in a
browser, on Windows, macOS and Linux alike. It comes with an AI assistant, a
data-science stack (data frames, arrays, charts), SQLite and MySQL, high-DPI
screens and accessibility, and more.

RapidR is an original implementation, written from the ground up in Rust.
Existing RapidQ programs run too.

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
time, "RapidR Studio" Not Opened appears: click Done, then System Settings >
Privacy & Security > Open Anyway (right-click > Open no longer works on
current macOS). On Windows, SmartScreen says "Windows protected your PC":
choose More info > Run anyway. Screenshots of each step: the manual's
"Getting started".

**Verify a download:** `SHA256SUMS` lists every file's SHA-256
(`shasum -a 256 -c SHA256SUMS`, or `Get-FileHash` on Windows).
`rapidr-@VERSION@.cdx.json` is the release's software bill of materials
(CycloneDX).

## Licences

RapidR is MIT-licensed, and everything it ships is under similarly
permissive open-source licences (Apache-2.0, BSD, ISC, zlib, BSL, Unicode,
OFL for the built-in fonts): nothing copyleft (GPL, LGPL, MPL, …) and no
TLS library is compiled into a program (HTTPS uses the operating
system's). **The programs you build with RapidR are
yours**, to ship as open source or commercially: every build writes a
`THIRD-PARTY-NOTICES.txt` beside the executable (or into the web build) with
every notice and licence text its components ask for — ship it with the
program, and that's all. See `LEGAL.md` (also trademarks, and no
warranty) and `docs/licensing.md`.

**RapidQ.** RapidR is an independent implementation that is compatible with
RapidQ. It contains nothing of RapidQ's — no code, manual text, include
files, examples or images — and doesn't include or distribute RapidQ. RapidQ's
own compiler is used only on the developer's test machine, as a black box
that runs test programs whose output RapidR is compared with. RapidR is not
affiliated with, endorsed or sponsored by RapidQ's author, anyone holding
rights in RapidQ, or any vendor it names; "RapidQ" is named only to say what
RapidR is compatible with (`LEGAL.md`, `docs/legal/rapidq-review.md`).

Every package carries `LICENSE`, `NOTICE`, `LEGAL.md`, `LICENSES.md`,
`THIRD_PARTY_NOTICES.md` and RapidR's own `THIRD-PARTY-NOTICES.txt`.

Built from commit @COMMIT@.

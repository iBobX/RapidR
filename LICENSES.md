# Third-Party Licenses

RapidR itself is licensed under the MIT License — see the top-level
[LICENSE](LICENSE) file. The redistributable artifacts that ship with
RapidR (the desktop binary, the in-browser IDE, and built web bundles)
include the third-party software listed below. Each entry names the
component, its upstream license, and how RapidR uses it.

If you redistribute RapidR or a built web bundle, please keep this file
alongside the binary and preserve the upstream copyright notices below.

---

## 1. Monaco Editor — MIT License

Vendored under `web-ide/vendor/monaco/` and used by the in-browser IDE for
source editing, syntax highlighting, and IntelliSense.

> Copyright (c) 2016 Microsoft Corporation. All rights reserved.
>
> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the "Software"), to deal
> in the Software without restriction, including without limitation the rights
> to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in
> all copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND.

Upstream: <https://github.com/microsoft/monaco-editor>

---

## 2. wasm-bindgen / wasm-pack generated glue — MIT OR Apache-2.0

`web-ide/runtime/rapidrintr.js` and `rapidrintr_bg.wasm` are produced by
`wasm-bindgen` from the `crates/rapidr-runtime-web` Rust crate. The
generated JavaScript wrapper inherits the wasm-bindgen license terms.

> Copyright (c) 2014 Alex Crichton
> Licensed under either of Apache-2.0 or MIT, at your option.

Upstream: <https://github.com/rustwasm/wasm-bindgen>

---

## 3. Rust standard library and Cargo dependencies

The compiled `rapidrintr_bg.wasm`, the desktop `rapidr` binary and the
runtimes linked into apps built with RapidR include the Rust standard
library (MIT OR Apache-2.0) and the open-source crates listed, with their
versions, licenses and upstream links, in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). That file is generated
from the real dependency graph by `tools/third_party_notices.py` and CI
fails if it is out of date; every license is checked against the permissive
allowlist in `deny.toml` (`cargo deny check licenses`).

---

## 4. Fonts

### Liberation fonts 2.1.5 — SIL Open Font License 1.1

`crates/rapidr-value/fonts/` holds Liberation Sans, Liberation Serif and
Liberation Mono (Regular), unmodified, from
<https://github.com/liberationfonts/liberation-fonts>. They are built into
the RapidR runtimes (desktop apps, the interpreter runner and the web
WebAssembly), which draw text on bitmaps with them; they have the same
character widths as Arial, Times New Roman and Courier New.

> Digitized data copyright (c) 2010 Google Corporation with Reserved Font
> Arimo, Tinos and Cousine. Copyright (c) 2012 Red Hat, Inc. with Reserved
> Font Name Liberation.
>
> This Font Software is licensed under the SIL Open Font License, Version
> 1.1. The full license text is in
> [`crates/rapidr-value/fonts/OFL-1.1.txt`](crates/rapidr-value/fonts/OFL-1.1.txt)
> and at <https://openfontlicense.org>.

### System fonts named by the IDE

The IDE references the system-installed `Inter`, `Tahoma`, `Arial`,
`Verdana`, `Times New Roman`, `Courier New`, `Segoe UI`, and
`MS Sans Serif` fonts via CSS only. Those font files are not vendored or
redistributed; end users supply them via the operating system or browser.

---

## 5. SQLite (via `rusqlite`) — Public Domain

RSQLITE is SQLite itself on every runtime (`crates/rapidr-db`). The
desktop runtime statically links it via `rusqlite` / `libsqlite3-sys`; the
web runtime's wasm (the IDE's and every web bundle's) compiles it in via
`rusqlite` / `sqlite-wasm-rs`. SQLite's source is in the public domain. See
<https://www.sqlite.org/copyright.html>.

---

## 6. PKZIP file format (in-browser exporter)

`web-ide/zip.js` is an original implementation of the PKZIP "stored"
format (no compression), written from the public PKZIP APPNOTE.TXT
specification and licensed under the same MIT terms as the rest of
RapidR. It is **not** derived from any GPL/LGPL ZIP library.

---

## 7. Native (C/C++) libraries compiled into or linked by RapidR

Some Rust crates wrap C/C++ libraries (`*-sys` crates). The wrapper crates
are listed in THIRD_PARTY_NOTICES.md under their own (permissive) licenses;
the native code they build or link has its own license, which `cargo deny`
cannot see, so it is credited here.

| Library | Via | License | Upstream |
|---|---|---|---|
| FreeType (font rendering for charts) | `freetype-sys` via `plotters` → `font-kit` | FreeType License (FTL) — see the notice below | <https://freetype.org> |
| Zstandard (zstd) | `zstd-sys` (Polars / Parquet) | BSD-3-Clause (dual GPL-2.0; used under BSD) | <https://github.com/facebook/zstd> |
| LZ4 | `lz4-sys` (Polars) | BSD-2-Clause | <https://github.com/lz4/lz4> |
| zlib | `libz-sys` | zlib License | <https://zlib.net> |
| SQLite | `libsqlite3-sys` (desktop), `sqlite-wasm-rs` (web wasm) | Public Domain (section 5) | <https://sqlite.org> |
| musl libc (the few C library functions SQLite needs in wasm) | `sqlite-wasm-rs` | MIT | <https://musl.libc.org> |
| printf (Marco Paland, Eyal Rozenberg) | `sqlite-wasm-rs` | MIT | <https://github.com/eyalroz/printf> |
| OpenSSL | `openssl-sys` (TLS for RHttp/sockets, linked from the system) | Apache-2.0 (OpenSSL 3) | <https://www.openssl.org> |
| Fontconfig (Linux) | `yeslogic-fontconfig-sys` (linked from the system; the charts' and the desktop UI's system fonts) | Fontconfig license (MIT-style) | <https://www.freedesktop.org/wiki/Software/fontconfig/> |
| X11, Wayland, xkbcommon (Linux) | `x11-dl`, `wayland-sys`, `xkbcommon-dl` (the desktop UI's windows and keys: loaded from the system when a window opens) | MIT / MIT-style | <https://www.x.org>, <https://wayland.freedesktop.org>, <https://xkbcommon.org> |
| ALSA (Linux), Core Audio (macOS) | `alsa-sys`, `coreaudio-sys` (system audio, linked) | LGPL-2.1 (alsa-lib, dynamically linked) / Apple system framework | — |
| SDL_GameControllerDB (data: the controller mappings) | `gilrs` (QDXJOYSTICK's gamepads on Windows and macOS; compiled in as text) | zlib License — Copyright © 1997-2025 Sam Lantinga | <https://github.com/gabomdq/SDL_GameControllerDB> |
| Windows.Gaming.Input (Windows), IOKit HID (macOS), the Linux kernel's evdev | `gilrs-core` (Windows, macOS); RapidR's own reader on Linux (system interfaces, no library linked) | system interfaces | — |

FreeType notice, as its license requires:

> Portions of this software are copyright © The FreeType Project
> (www.freetype.org). All rights reserved.

### 7.1 Windows: LLVM-MinGW, shipped with the RapidR SDK

The Windows SDK ships a trimmed [LLVM-MinGW](https://github.com/mstorsjo/llvm-mingw)
(`lib\rapidr\toolchain\`, the release it came from in its `README.txt`), and
RapidR's own Windows executables are built with it, through Rust's
`*-pc-windows-gnullvm` targets. Native builds (`rapidr build`) link with it,
so they need no Visual Studio.

What ships, and its licences:

| Part | What it is | Licence |
|---|---|---|
| clang, lld, llvm-ar and the LLVM libraries they load | the compiler and linker (tools: nothing of them is in a built program) | Apache-2.0 WITH LLVM-exception (`toolchain\LICENSE.TXT`) |
| compiler-rt builtins, libunwind | linked **statically** into every built program (Rust's unwinder and runtime helpers; `+crt-static`) | Apache-2.0 WITH LLVM-exception: the LLVM exception waives the attribution in binaries |
| mingw-w64 runtime (crt objects, `libmingw32`, `libmingwex`, …) | linked statically into every built program | ZPL-2.1, with parts under BSD / MIT / ISC-style and public-domain terms (`toolchain\<arch>-w64-mingw32\share\mingw32\COPYING.MinGW-w64-runtime.txt`) |
| mingw-w64 headers | used while compiling C code the crates carry (SQLite, …) | ZPL-2.1 / public domain; a few headers and IDLs imported from Wine are LGPL-2.1+ |
| libc++, winpthreads, libomp | in the toolchain, **not** linked into Rust programs (no C++ in them) | Apache-2.0 WITH LLVM-exception / MIT and BSD |

Nothing GPL is linked: there is no libgcc, libstdc++ or GNU binutils in
LLVM-MinGW (compiler-rt and libunwind take libgcc's place). The release
scripts check what RapidR's executables import (Windows' own DLLs only), and
the Windows smoke test checks the same of a program built natively.

**What this means for programs built with RapidR on Windows** (native builds;
interpreted executables and the runtime are RapidR's own, MIT):

- The mingw-w64 runtime is in the executable. Its licence asks for its
  copyright notices to be reproduced with binary distributions (BSD / ZPL
  style: a notice, nothing more). Ship
  `COPYING.MinGW-w64-runtime.txt` (in the SDK's toolchain folder, above) with
  your program, or its notices in your documentation. Open source or
  commercial, either is fine.
- The LGPL-2.1 Wine headers are only ever *compiled against* (declarations,
  constants, small inline functions); LGPL-2.1 §5 leaves such a program's
  licence to its author. No LGPL code is linked.
- With `RAPIDR_TOOLCHAIN=msvc` (Microsoft's C++ Build Tools instead) the
  program carries Microsoft's C runtime under Visual Studio's redistribution
  terms instead.

ALSA (Linux, above) is LGPL-2.1 and linked **dynamically** from the user's
system (`libasound.so.2`), never shipped: that keeps RapidR's and its
programs' licences their own.

---

## 8. RapidR (this project) — MIT License

See [LICENSE](LICENSE).

This includes the media code and data built into programs: QMIDI's
built-in General MIDI synthesizer (`rapidr-value/src/objects/synth.rs`)
generates its instruments procedurally — no SoundFont or sample data is
bundled — and QVIDEO's AVI reader and video decoders (uncompressed DIB,
RLE8 / RLE4, Microsoft Video 1, Cinepak; Motion JPEG through the
`jpeg-decoder` crate listed in THIRD_PARTY_NOTICES.md) are original code
written from the formats' public descriptions, not derived from FFmpeg,
libav, GStreamer or any other GPL/LGPL project. The AVI and MIDI test
fixtures are generated by RapidR's own scripts (`tools/make_avi_fixtures.py`,
`tools/make_media_fixture.py`).

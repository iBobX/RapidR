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

---

## 8. RapidR (this project) — MIT License

See [LICENSE](LICENSE).

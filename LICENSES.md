# Third-Party Licenses

RapidR itself is licensed under the MIT License — see the top-level
[LICENSE](LICENSE) file. The redistributable artifacts that ship with
RapidR (the desktop binary, the in-browser IDE, and built web bundles)
include the third-party software listed below. Each entry names the
component, its upstream license, and how RapidR uses it.

**Programs you build** don't need this file: each one gets its own
`THIRD-PARTY-NOTICES.txt` (beside the executable, or in the web build's
folder), generated from that build's components with every licence text in
full. Ship it with the program; that is all. [LEGAL.md](LEGAL.md) says what
you may do with RapidR and your programs; [docs/licensing.md](docs/licensing.md)
has the details per output.

If you redistribute RapidR itself, keep this file, `THIRD_PARTY_NOTICES.md`
and `THIRD-PARTY-NOTICES.txt` (in an install's `share/doc/rapidr/`) with it.

The VS Code extension (`rapidr-<version>.vsix`, MIT) carries its own
`THIRD_PARTY_NOTICES.md`: the JavaScript packages bundled into it
(`vscode-languageclient` and its dependencies, MIT / ISC), with their licence
texts (`utilities/vscodeext/rapidr/scripts/notices.js` makes it).

---

## 1. (removed) Monaco Editor

The Monaco editor (MIT, with DOMPurify, marked and the Codicons font inside
it) was vendored for the old HTML in-browser IDE. That IDE was deleted on
2026-10-08 (RapidR Studio, drawn by the UI kernel, replaces it); nothing of
Monaco is in the source tree, in a release package or in a program you
build. (The section keeps its number so that references to the others stay
right.)

---

## 2. wasm-bindgen / wasm-pack generated glue — MIT OR Apache-2.0

The web runtime's `rapidrintr.js` and `rapidrintr_bg.wasm` (`target/web`:
in RapidR Studio's web build and in every web bundle) are produced by
`wasm-bindgen` from the `interpreter/rapidr-vm-host-web` Rust crate. The
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
from the real dependency graph by `tools/third_party_notices.py`
(`--check` fails if it is out of date); every license is checked against the
permissive allowlist in `deny.toml` (`cargo deny check licenses`). The full
licence texts, per kind of output, are in the `THIRD-PARTY-NOTICES.txt` files
`rapidr notices` generates (crates/rapidr-cli/src/notices.rs).

---

## 4. Fonts

### Inter 4.1 and JetBrains Mono 2.304 — SIL Open Font License 1.1

`crates/rapidr-value/fonts/` also holds Inter (Regular, SemiBold) and
JetBrains Mono (Regular, Bold), subset to the Latin scripts and unhinted by
`tools/fonts/subset_ui_fonts.py` from
<https://github.com/rsms/inter/releases/tag/v4.1> and
<https://github.com/JetBrains/JetBrainsMono/releases/tag/v2.304> (the
archives' SHA-256 in the script). They are RapidR's own UI and code faces
(docs/ide-plan.md decision D8): RapidR Studio's chrome and code editor, the
RapidR look's menus and tooltips; programs can name them. Neither declares a
Reserved Font Name, so the subsets keep their names.

> Copyright (c) 2016 The Inter Project Authors (https://github.com/rsms/inter).
> Copyright 2020 The JetBrains Mono Project Authors
> (https://github.com/JetBrains/JetBrainsMono).
>
> This Font Software is licensed under the SIL Open Font License, Version
> 1.1: [`crates/rapidr-value/fonts/Inter-OFL.txt`](crates/rapidr-value/fonts/Inter-OFL.txt),
> [`crates/rapidr-value/fonts/JetBrainsMono-OFL.txt`](crates/rapidr-value/fonts/JetBrainsMono-OFL.txt).

### Liberation fonts 2.1.5 — SIL Open Font License 1.1

`crates/rapidr-value/fonts/` holds Liberation Sans, Liberation Serif and
Liberation Mono (Regular), unmodified, from
<https://github.com/liberationfonts/liberation-fonts>. They are built into
the RapidR runtimes (desktop apps, the interpreter runner and the web
WebAssembly), which draw text on bitmaps with them; they have the same
character widths as Arial, Times New Roman and Courier New.

Their designed Bold, Italic and Bold Italic faces (Liberation 2.1.5's own,
from the same official release archive, SHA-256
`7191c669bf38899f73a2094ed00f7b800553364f90e2637010a69c0e268f25d0`) are built
in too, as nine Modified Versions cut to the Latin scripts without hinting
(36 to 39 KB each) and renamed **RapidR Text Sans**, **RapidR Text Serif** and
**RapidR Text Mono** as the OFL requires (Liberation, Arimo, Tinos and
Cousine are Reserved Font Names; their copyright lines and the licence are
kept in each file). The glyph outlines are Liberation's, unchanged;
`tools/fonts/make_liberation_styles.py` makes the files reproducibly. With
them, bold and italic text is drawn from designed faces as wide as Arial
Bold's, Times New Roman Bold's and Courier New Bold's instead of the Regular
letters drawn heavier.

Beside them, **RapidR Sans** (`RapidRSans-Regular.ttf`, `RapidRSans-Bold.ttf`) is a Modified
Version of Liberation Sans under the same licence, renamed as the OFL
requires (it carries none of the Reserved Font Names): Liberation Sans with
each Windows-1252 character as wide as MS Sans Serif's at 8 pt (a few a
pixel wider, so its letters don't run together) and MS Sans Serif's line
metrics — the face RapidR draws RapidQ's default font with.
`tools/fonts/make_rapidr_sans.py` makes it; the widths are measurements of
RapidQ's `TextWidth`, no Microsoft font data
(`crates/rapidr-value/fonts/README.md`).

> Digitized data copyright (c) 2010 Google Corporation with Reserved Font
> Arimo, Tinos and Cousine. Copyright (c) 2012 Red Hat, Inc. with Reserved
> Font Name Liberation.
>
> This Font Software is licensed under the SIL Open Font License, Version
> 1.1. The full license text is in
> [`crates/rapidr-value/fonts/OFL-1.1.txt`](crates/rapidr-value/fonts/OFL-1.1.txt)
> and at <https://openfontlicense.org>.

### Cantarell Regular — SIL Open Font License 1.1 (Linux)

Built into the `sctk-adwaita` crate, through which winit draws a window's
title bar on Wayland desktops that leave it to the program (GNOME):
unmodified, its fallback face for the title. Copyright (c) 2009-2011,
Understanding Limited; Copyright (c) 2010-2011, Jakub Steiner. Its notice
and the OFL's text are in the Linux programs' `THIRD-PARTY-NOTICES.txt`.

### Noto fallback fonts — SIL Open Font License 1.1

What the Liberation fonts lack (symbols such as ✓, Chinese, Japanese,
Korean) is drawn with Noto fonts by the Noto Project Authors. They come from
the Noto project's official GitHub release assets, pinned by version and
SHA-256 in [`fonts/fallback/fonts.toml`](fonts/fallback/fonts.toml):
- Noto Sans 2.015;
- Noto Sans Symbols 2.003;
- Noto Sans Symbols 2 2.008;
- Noto Sans SC and Noto Sans KR from Noto CJK Sans 2.004;
- Noto Color Emoji (COLRv1) from the noto-emoji repository at its release tag v2.051.

**Where they are.** Noto Sans and the two Symbols fonts are in
`fonts/fallback/`, unmodified. The CJK fonts are fetched at build time
(`tools/fonts.py`).

**What is shipped.** `tools/fonts.py` splits the fonts by Unicode range into
the chunks the web runtime loads on demand: the runtime's `fonts/` folder,
web bundles and `rapidr build --web` sites. A chunk of a CJK font is renamed
`<family> NNN` (for example `Noto Sans SC 003`) so each loads as a family of
its own. This subsetting and renaming are allowed by the OFL; none of these
fonts declares a Reserved Font Name. `OFL.txt` travels with the chunks.

> Noto Sans, Noto Sans Symbols, Noto Sans Symbols 2: Copyright 2022 The
> Noto Project Authors (https://github.com/notofonts/latin-greek-cyrillic,
> https://github.com/notofonts/symbols). Noto Sans SC, Noto Sans KR (Noto
> Sans CJK): © 2014-2021 Adobe (http://www.adobe.com/). Noto Color Emoji:
> Copyright 2022 Google Inc.
>
> This Font Software is licensed under the SIL Open Font License, Version
> 1.1. The full license text is in
> [`fonts/fallback/OFL.txt`](fonts/fallback/OFL.txt) and at
> <https://openfontlicense.org>.

### Font names in the IDE

The IDE's own interface uses the system's font stack. Its font picker lists
`Inter`, `Roboto`, `Montserrat`, `Nunito`, `Playfair Display`, `Fira Code`,
`Tahoma`, `Arial`, `Verdana`, `Times New Roman`, `Courier New`, `Segoe UI`
and `MS Sans Serif` as names a program may ask for. They are names only: no
font file is vendored, redistributed or downloaded (no web-font service is
contacted). A program's text is drawn with RapidR's own fonts above, the
named families resolving through their fallback.

---

## 5. SQLite (via `rusqlite`) — Public Domain

RSQLITE is SQLite itself on every runtime (`crates/rapidr-db`). The
desktop runtime statically links it via `rusqlite` / `libsqlite3-sys`; the
web runtime's wasm (the IDE's and every web bundle's) compiles it in via
`rusqlite` / `sqlite-wasm-rs`. SQLite's source is in the public domain. See
<https://www.sqlite.org/copyright.html>.

---

## 6. (removed) PKZIP exporter

The old HTML in-browser IDE had its own PKZIP "stored" writer (an original
implementation, MIT); it was deleted with that IDE on 2026-10-08. (The
section keeps its number so that references to the others stay right.)

---

## 7. Native (C/C++) libraries compiled into or linked by RapidR

Some Rust crates wrap C/C++ libraries (`*-sys` crates). The wrapper crates
are listed in THIRD_PARTY_NOTICES.md under their own (permissive) licenses;
the native code they build or link has its own license, which `cargo deny`
cannot see, so it is credited here.

| Library | Via | License | Upstream |
|---|---|---|---|
| SQLite | `libsqlite3-sys` (desktop), `sqlite-wasm-rs` (web wasm) | Public Domain (section 5) | <https://sqlite.org> |
| musl libc (the few C library functions SQLite needs in wasm) | `sqlite-wasm-rs` | MIT | <https://musl.libc.org> |
| printf (Marco Paland, Eyal Rozenberg) | `sqlite-wasm-rs` | MIT | <https://github.com/eyalroz/printf> |
| The operating system's TLS: Security.framework (macOS), SChannel (Windows), OpenSSL 3 (Linux: `libssl.so.3` / `libcrypto.so.3`) | `native-tls` (with `security-framework`, `schannel`, `openssl`; HTTPS for RHTTP and QDOWNLOAD) | system components; OpenSSL 3 is Apache-2.0 — linked dynamically, never shipped | <https://www.openssl.org> |
| Fontconfig (Linux) | `yeslogic-fontconfig-sys` (linked from the system; the desktop UI's system fonts) | Fontconfig license (MIT-style) | <https://www.freedesktop.org/wiki/Software/fontconfig/> |
| X11, Wayland, xkbcommon (Linux) | `x11-dl`, `wayland-sys`, `xkbcommon-dl` (the desktop UI's windows and keys: loaded from the system when a window opens) | MIT / MIT-style | <https://www.x.org>, <https://wayland.freedesktop.org>, <https://xkbcommon.org> |
| ALSA (Linux), Core Audio (macOS) | `alsa-sys`, `coreaudio-sys` (system audio, linked) | LGPL-2.1 (alsa-lib, dynamically linked) / Apple system framework | — |
| SDL_GameControllerDB (data: the controller mappings) | `gilrs` (QDXJOYSTICK's gamepads on Windows and macOS; compiled in as text) | zlib License — Copyright © 1997-2025 Sam Lantinga | <https://github.com/gabomdq/SDL_GameControllerDB> |
| Windows.Gaming.Input (Windows), IOKit HID (macOS), the Linux kernel's evdev | `gilrs-core` (Windows, macOS); RapidR's own reader on Linux (system interfaces, no library linked) | system interfaces | — |

Data and protocol descriptions compiled into crates (their notices are in
every generated `THIRD-PARTY-NOTICES.txt` that needs them):

| What | Via | License |
|---|---|---|
| Wayland protocol descriptions (`wayland.xml`, wayland-protocols, wlr-protocols) | `wayland-client`, `wayland-protocols`, `wayland-protocols-wlr` (Linux): the Rust code is generated from them | MIT, and HPND-sell-variant (MIT's older X11 kin: a notice, no endorsement) |
| xcb-proto (the X11 protocol descriptions) | `x11rb-protocol` (Linux), generated from them | X11 (MIT with a no-advertising clause) |
| Adobe Glyph List | `read-fonts` (through `parley`): its table of glyph names | BSD-3-Clause, © Adobe |
| KDE's Wayland blur protocol | **not used**: crates.io's `wayland-protocols-plasma` (generated from KDE's protocol files, some LGPL-2.1-or-later) is replaced by RapidR's own stand-in, `crates/patches/wayland-protocols-plasma` (MIT, its own protocol file) | — |

No cryptographic library is compiled into anything RapidR builds: HTTPS
uses the operating system's TLS (above). Zstandard, LZ4, zlib and OpenSSL's
sources appear in `Cargo.lock` only as optional or other-platform
dependencies of crates RapidR uses and are compiled into nothing RapidR
ships: the generated `THIRD-PARTY-NOTICES.txt` files, made from the real
graphs, list what is.

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
| libunwind | linked **statically** into every built program (Rust's unwinder; `+crt-static`) | Apache-2.0 WITH LLVM-exception: the LLVM exception waives the attribution in binaries |
| compiler-rt builtins | in the toolchain, **not** linked into Rust programs (Rust's `compiler_builtins` provides them) | Apache-2.0 WITH LLVM-exception |
| mingw-w64 runtime (crt objects, `libmingw32`, `libmingwex`, …) | a few of its objects linked statically into every built program: start-up code, UCRT shims, a few math helpers; on x64 also its `fprintf` with David M. Gay's gdtoa (docs/licensing.md §3.3 lists them, from a link map) | the linked ones: public domain, ZPL-2.1 and gdtoa's permission notice (HPND); the whole runtime: ZPL-2.1, with parts under BSD / MIT / ISC-style and public-domain terms (`toolchain\<arch>-w64-mingw32\share\mingw32\COPYING.MinGW-w64-runtime.txt`) |
| mingw-w64 headers | used while compiling C code the crates carry (SQLite, …) | ZPL-2.1 / public domain; a few headers and IDLs imported from Wine are LGPL-2.1+ |
| libc++, winpthreads, libomp | in the toolchain, **not** linked into Rust programs (no C++ in them) | Apache-2.0 WITH LLVM-exception / MIT and BSD |

Nothing GPL is linked: there is no libgcc, libstdc++ or GNU binutils in
LLVM-MinGW (compiler-rt and libunwind take libgcc's place). The release
scripts check what RapidR's executables import (Windows' own DLLs only), and
the Windows smoke test checks the same of a program built natively.

**What this means for programs built with RapidR on Windows** (native builds;
interpreted executables and the runtime are RapidR's own, MIT):

- Parts of the mingw-w64 runtime are in the executable. Their licences
  (ZPL-2.1, and gdtoa's notice on x64) ask for their copyright notices to be
  reproduced with binary distributions (a notice, nothing more). The
  `THIRD-PARTY-NOTICES.txt` that `rapidr build` writes beside the program
  carries exactly those notices: ship it with your program. Nothing
  Cephes-derived, nothing imported from Wine and nothing (L)GPL is linked
  (docs/licensing.md §3.3). Open source or commercial, either is fine.
- The LGPL-2.1 Wine headers are only ever *compiled against* (declarations,
  constants, small inline functions); LGPL-2.1 §5 leaves such a program's
  licence to its author. No LGPL code is linked.
- With `RAPIDR_TOOLCHAIN=msvc` (Microsoft's C++ Build Tools instead) the
  program carries Microsoft's C runtime under Visual Studio's redistribution
  terms instead.

**The texts**, from the LLVM-MinGW release the SDK ships (also in
`crates/rapidr-cli/licenses/`, from which `rapidr notices` writes them into
every Windows build's `THIRD-PARTY-NOTICES.txt`). compiler-rt and libunwind
are under LLVM's licence, Apache-2.0 WITH LLVM-exception
(`crates/rapidr-cli/licenses/Apache-2.0-WITH-LLVM-exception.txt`, and
`toolchain\LICENSE.TXT` in the SDK).

<details><summary>mingw-w64 — COPYING (ZPL-2.1)</summary>

```text
With exception of certain parts that are prominently marked as being
in the Public Domain, BSD, or LGPL this Software is provided under the
Zope Public License (ZPL) Version 2.1.

Copyright (c) 2009 - 2013 by the mingw-w64 project

See the AUTHORS file for the list of contributors to the mingw-w64 project.

This license has been certified as open source. It has also been designated
as GPL compatible by the Free Software Foundation (FSF).

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

   1. Redistributions in source code must retain the accompanying copyright
      notice, this list of conditions, and the following disclaimer.
   2. Redistributions in binary form must reproduce the accompanying
      copyright notice, this list of conditions, and the following disclaimer
      in the documentation and/or other materials provided with the
      distribution.
   3. Names of the copyright holders must not be used to endorse or promote
      products derived from this software without prior written permission
      from the copyright holders.
   4. The right to distribute this software or to use it for any purpose does
      not give you the right to use Servicemarks (sm) or Trademarks (tm) of
      the copyright holders.  Use of them is covered by separate agreement
      with the copyright holders.
   5. If any files are modified, you must cause the modified files to carry
      prominent notices stating that you changed the files and the date of
      any change.

Disclaimer

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS ``AS IS'' AND ANY EXPRESSED
OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO
EVENT SHALL THE COPYRIGHT HOLDERS BE LIABLE FOR ANY DIRECT, INDIRECT,
INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, 
OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE,
EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

</details>

<details><summary>mingw-w64 — COPYING.MinGW-w64-runtime.txt (the notices the runtime asks for)</summary>

```text
MinGW-w64 runtime licensing
***************************

This program or library was built using MinGW-w64 and statically
linked against the MinGW-w64 runtime. Some parts of the runtime
are under licenses which require that the copyright and license
notices are included when distributing the code in binary form.
These notices are listed below.


========================
Overall copyright notice
========================

Copyright (c) 2009, 2010, 2011, 2012, 2013 by the mingw-w64 project

This license has been certified as open source. It has also been designated
as GPL compatible by the Free Software Foundation (FSF).

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

   1. Redistributions in source code must retain the accompanying copyright
      notice, this list of conditions, and the following disclaimer.
   2. Redistributions in binary form must reproduce the accompanying
      copyright notice, this list of conditions, and the following disclaimer
      in the documentation and/or other materials provided with the
      distribution.
   3. Names of the copyright holders must not be used to endorse or promote
      products derived from this software without prior written permission
      from the copyright holders.
   4. The right to distribute this software or to use it for any purpose does
      not give you the right to use Servicemarks (sm) or Trademarks (tm) of
      the copyright holders.  Use of them is covered by separate agreement
      with the copyright holders.
   5. If any files are modified, you must cause the modified files to carry
      prominent notices stating that you changed the files and the date of
      any change.

Disclaimer

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS ``AS IS'' AND ANY EXPRESSED
OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO
EVENT SHALL THE COPYRIGHT HOLDERS BE LIABLE FOR ANY DIRECT, INDIRECT,
INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA,
OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE,
EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

======================================== 
getopt, getopt_long, and getop_long_only
======================================== 

Copyright (c) 2002 Todd C. Miller <Todd.Miller@courtesan.com> 
 
Permission to use, copy, modify, and distribute this software for any 
purpose with or without fee is hereby granted, provided that the above 
copyright notice and this permission notice appear in all copies. 
 	 
THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

Sponsored in part by the Defense Advanced Research Projects
Agency (DARPA) and Air Force Research Laboratory, Air Force
Materiel Command, USAF, under agreement number F39502-99-1-0512.

        *       *       *       *       *       *       * 

Copyright (c) 2000 The NetBSD Foundation, Inc.
All rights reserved.

This code is derived from software contributed to The NetBSD Foundation
by Dieter Baron and Thomas Klausner.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions
are met:
 1. Redistributions of source code must retain the above copyright
    notice, this list of conditions and the following disclaimer.
 2. Redistributions in binary form must reproduce the above copyright
    notice, this list of conditions and the following disclaimer in the
    documentation and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE NETBSD FOUNDATION, INC. AND CONTRIBUTORS
``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE FOUNDATION OR CONTRIBUTORS
BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
POSSIBILITY OF SUCH DAMAGE.


===============================================================
gdtoa: Converting between IEEE floating point numbers and ASCII
===============================================================

The author of this software is David M. Gay.

Copyright (C) 1997, 1998, 1999, 2000, 2001 by Lucent Technologies
All Rights Reserved

Permission to use, copy, modify, and distribute this software and
its documentation for any purpose and without fee is hereby
granted, provided that the above copyright notice appear in all
copies and that both that the copyright notice and this
permission notice and warranty disclaimer appear in supporting
documentation, and that the name of Lucent or any of its entities
not be used in advertising or publicity pertaining to
distribution of the software without specific, written prior
permission.

LUCENT DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE,
INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS.
IN NO EVENT SHALL LUCENT OR ANY OF ITS ENTITIES BE LIABLE FOR ANY
SPECIAL, INDIRECT OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER
IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF
THIS SOFTWARE.

        *       *       *       *       *       *       *

The author of this software is David M. Gay.

Copyright (C) 2005 by David M. Gay
All Rights Reserved

Permission to use, copy, modify, and distribute this software and its
documentation for any purpose and without fee is hereby granted,
provided that the above copyright notice appear in all copies and that
both that the copyright notice and this permission notice and warranty
disclaimer appear in supporting documentation, and that the name of
the author or any of his current or former employers not be used in
advertising or publicity pertaining to distribution of the software
without specific, written prior permission.

THE AUTHOR DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE,
INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS.  IN
NO EVENT SHALL THE AUTHOR OR ANY OF HIS CURRENT OR FORMER EMPLOYERS BE
LIABLE FOR ANY SPECIAL, INDIRECT OR CONSEQUENTIAL DAMAGES OR ANY
DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS,
WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS
SOFTWARE.

        *       *       *       *       *       *       *

The author of this software is David M. Gay.

Copyright (C) 2004 by David M. Gay.
All Rights Reserved
Based on material in the rest of /netlib/fp/gdota.tar.gz,
which is copyright (C) 1998, 2000 by Lucent Technologies.

Permission to use, copy, modify, and distribute this software and
its documentation for any purpose and without fee is hereby
granted, provided that the above copyright notice appear in all
copies and that both that the copyright notice and this
permission notice and warranty disclaimer appear in supporting
documentation, and that the name of Lucent or any of its entities
not be used in advertising or publicity pertaining to
distribution of the software without specific, written prior
permission.

LUCENT DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE,
INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS.
IN NO EVENT SHALL LUCENT OR ANY OF ITS ENTITIES BE LIABLE FOR ANY
SPECIAL, INDIRECT OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER
IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF
THIS SOFTWARE.


=========================
Parts of the math library
=========================

Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.

Developed at SunSoft, a Sun Microsystems, Inc. business.
Permission to use, copy, modify, and distribute this
software is freely granted, provided that this notice
is preserved.

        *       *       *       *       *       *       *

Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.

Developed at SunPro, a Sun Microsystems, Inc. business.
Permission to use, copy, modify, and distribute this
software is freely granted, provided that this notice
is preserved.

        *       *       *       *       *       *       *

FIXME: Cephes math lib
Copyright (C) 1984-1998 Stephen L. Moshier

It sounds vague, but as to be found at
<http://lists.debian.org/debian-legal/2004/12/msg00295.html>, it gives an
impression that the author could be willing to give an explicit
permission to distribute those files e.g. under a BSD style license. So
probably there is no problem here, although it could be good to get a
permission from the author and then add a license into the Cephes files
in MinGW runtime. At least on follow-up it is marked that debian sees the
version a-like BSD one. As MinGW.org (where those cephes parts are coming
from) distributes them now over 6 years, it should be fine.

=================================================
Some string, memory and time conversion functions
=================================================

Copyright © 2005-2020 Rich Felker, et al.

Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the
"Software"), to deal in the Software without restriction, including
without limitation the rights to use, copy, modify, merge, publish,
distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to
the following conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

===================================
Headers and IDLs imported from Wine
===================================

Some header and IDL files were imported from the Wine project. These files
are prominent maked in source. Their copyright belongs to contributors and
they are distributed under LGPL license.

Disclaimer

This library is free software; you can redistribute it and/or
modify it under the terms of the GNU Lesser General Public
License as published by the Free Software Foundation; either
version 2.1 of the License, or (at your option) any later version.

This library is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
Lesser General Public License for more details.
```

</details>

ALSA (Linux, above) is LGPL-2.1 and linked **dynamically** from the user's
system (`libasound.so.2`), never shipped: that keeps RapidR's and its
programs' licences their own.

---

## 8. RapidR (this project) — MIT License

See [LICENSE](LICENSE). Trademarks and affiliation: RapidR is compatible with
RapidQ and is not affiliated with RapidQ's author or any vendor it names
(see [LEGAL.md](LEGAL.md)).

This includes the media code and data built into programs: QMIDI's
built-in General MIDI synthesizer (`rapidr-value/src/objects/synth.rs`)
generates its instruments procedurally — no SoundFont or sample data is
bundled — and QVIDEO's AVI reader and video decoders (uncompressed DIB,
RLE8 / RLE4, Microsoft Video 1, Cinepak; Motion JPEG through the
`jpeg-decoder` crate listed in THIRD_PARTY_NOTICES.md) are original code
written from the formats' public descriptions, not derived from FFmpeg,
libav, GStreamer or any other GPL/LGPL project. MP3 files play on the desktop
through `nanomp3` (MIT OR Apache-2.0, a pure-Rust port of the
public-domain minimp3; MP3's patents have expired); no H.264, HEVC, AAC or other patent-encumbered codec is
included (docs/licensing.md §5). The AVI and MIDI test
fixtures are generated by RapidR's own scripts (`tools/make_avi_fixtures.py`,
`tools/make_media_fixture.py`).

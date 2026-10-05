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

---

## 1. Monaco Editor 0.52.2 — MIT License

Vendored under `web-ide/vendor/monaco/` and used by the in-browser IDE (only
the IDE: never a program you build) for source editing, syntax highlighting,
and IntelliSense.

> The MIT License (MIT)
>
> Copyright (c) 2016 - present Microsoft Corporation
>
> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the "Software"), to deal
> in the Software without restriction, including without limitation the rights
> to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all
> copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
> SOFTWARE.

Upstream: <https://github.com/microsoft/monaco-editor>

Monaco's `editor.main.js` bundles, under their own licences:

| Component | License | Upstream |
|---|---|---|
| DOMPurify 3.1.7 (© Cure53 and other contributors) | Apache-2.0 or MPL-2.0, at your option (used under Apache-2.0) | <https://github.com/cure53/DOMPurify> |
| marked (© Christopher Jeffrey and the Marked contributors) | MIT | <https://github.com/markedjs/marked> |
| Codicons icon font (`codicon.ttf`, © Microsoft Corporation) | CC-BY-4.0 (the icons) | <https://github.com/microsoft/vscode-codicons> |

Their full texts are the standard ones: Apache-2.0
(<https://www.apache.org/licenses/LICENSE-2.0>), MIT (above), CC-BY-4.0
(<https://creativecommons.org/licenses/by/4.0/legalcode>). The codicons are
credited here as CC-BY-4.0 asks (author, licence, link; unmodified).

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
from the real dependency graph by `tools/third_party_notices.py`
(`--check` fails if it is out of date); every license is checked against the
permissive allowlist in `deny.toml` (`cargo deny check licenses`). The full
licence texts, per kind of output, are in the `THIRD-PARTY-NOTICES.txt` files
`rapidr notices` generates (crates/rapidr-cli/src/notices.rs).

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

### Fonts the web IDE names or loads

The web IDE's pages (`web-ide/index.html`, `preview.html`) load Inter,
Roboto, Montserrat, Nunito, Playfair Display and Fira Code from Google Fonts
(SIL OFL 1.1 / Apache-2.0); Google serves them, RapidR doesn't redistribute
them, and programs you build don't use them. The IDE also names system fonts
(`Tahoma`, `Arial`, `Verdana`, `Times New Roman`, `Courier New`, `Segoe UI`,
`MS Sans Serif`) in CSS only: the operating system supplies those.

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
| FreeType (font rendering for charts, Linux) | `freetype-sys` via `plotters` → `font-kit` (the system's library when installed, else built in) | FreeType License (FTL) — see the notice below | <https://freetype.org> |
| SQLite | `libsqlite3-sys` (desktop), `sqlite-wasm-rs` (web wasm) | Public Domain (section 5) | <https://sqlite.org> |
| musl libc (the few C library functions SQLite needs in wasm) | `sqlite-wasm-rs` | MIT | <https://musl.libc.org> |
| printf (Marco Paland, Eyal Rozenberg) | `sqlite-wasm-rs` | MIT | <https://github.com/eyalroz/printf> |
| BoringSSL-derived C and assembly | `ring` (TLS for HTTPS through rustls, macOS and Linux) | Apache-2.0 / ISC (ring's licence files) | <https://github.com/briansmith/ring> |
| Windows' TLS (Schannel) | `native-tls` / `schannel` (HTTPS on Windows: a system component) | system interface | — |
| Fontconfig (Linux) | `yeslogic-fontconfig-sys` (linked from the system; the charts' and the desktop UI's system fonts) | Fontconfig license (MIT-style) | <https://www.freedesktop.org/wiki/Software/fontconfig/> |
| X11, Wayland, xkbcommon (Linux) | `x11-dl`, `wayland-sys`, `xkbcommon-dl` (the desktop UI's windows and keys: loaded from the system when a window opens) | MIT / MIT-style | <https://www.x.org>, <https://wayland.freedesktop.org>, <https://xkbcommon.org> |
| ALSA (Linux), Core Audio (macOS) | `alsa-sys`, `coreaudio-sys` (system audio, linked) | LGPL-2.1 (alsa-lib, dynamically linked) / Apple system framework | — |
| SDL_GameControllerDB (data: the controller mappings) | `gilrs` (QDXJOYSTICK's gamepads on Windows and macOS; compiled in as text) | zlib License — Copyright © 1997-2025 Sam Lantinga | <https://github.com/gabomdq/SDL_GameControllerDB> |
| Windows.Gaming.Input (Windows), IOKit HID (macOS), the Linux kernel's evdev | `gilrs-core` (Windows, macOS); RapidR's own reader on Linux (system interfaces, no library linked) | system interfaces | — |

FreeType notice, as its license requires:

> Portions of this software are copyright © The FreeType Project
> (www.freetype.org). All rights reserved.

Zstandard, LZ4, zlib and OpenSSL appear in `Cargo.lock` (optional or
other-platform dependencies of crates RapidR uses) but are compiled into
nothing RapidR ships: the generated `THIRD-PARTY-NOTICES.txt` files, made
from the real graphs, list what is.

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
through `rodio`'s decoder (`symphonia`, MPL-2.0, unmodified; MP3's patents
have expired); no H.264, HEVC, AAC or other patent-encumbered codec is
included (docs/licensing.md §5). The AVI and MIDI test
fixtures are generated by RapidR's own scripts (`tools/make_avi_fixtures.py`,
`tools/make_media_fixture.py`).

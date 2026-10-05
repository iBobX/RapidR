# Licensing: what is inside each output, and what its licences ask

The plain-language version is [LEGAL.md](../LEGAL.md). This page has the
facts behind it: what RapidR compiles, links or ships into each kind of
output, under which licences, and what those licences ask of someone who
distributes a program or a web app. It documents licence terms as the project
reads them; it is not legal advice (see "Open questions" at the end, and have
the whole reviewed by a professional before the first public release).

## 1. The notices every output carries

Every output RapidR builds gets a `THIRD-PARTY-NOTICES.txt`:

| Output | Command | Where the notices go |
|---|---|---|
| Native desktop executable | `rapidr build x.bas` | beside the executable |
| Interpreted desktop executable | `rapidr build x.bas --interp [--target <os>-<arch>]` | beside the executable |
| Web bundle (bytecode) | `rapidr bundle-bc x.bas`, `rapidr build --web --interp`, the web IDE's **Build** | in the zip's root; `index.html` has a comment and a `<link rel="license">` to it |
| Native web build | `rapidr build x.bas --web` | in `x_web/`, linked the same way |
| Bytecode | `rapidr build-bc x.bas` | none needed: a `.rrbc` holds only the program; the RapidR Runtime that runs it carries its own notices |
| RapidR itself | the installers | `share/doc/rapidr/THIRD-PARTY-NOTICES.txt` (plus `LICENSE`, `LEGAL.md`, `LICENSES.md`, `THIRD_PARTY_NOTICES.md`) |

The file has two parts: the components (name, version, licence as declared,
the licence it is used under when there is a choice, upstream link, and notes
such as where the source of an MPL-2.0 file is), and every licence text and
notice in full, each identical text once, with the components it belongs to.

**One generator.** `crates/rapidr-cli/src/notices.rs`. The text depends on
the kind of output, never on the program, so programs for the same system
share one file:

| Kind (`rapidr notices <kind>`) | Workspace crates whose graph it covers | Rust targets |
|---|---|---|
| `<os>-<arch>` (desktop) | `rapidr-runtime-core` (native programs) + `rapidr-runner-stub` (interpreted) | that target; Windows: both `*-pc-windows-gnullvm` and `*-pc-windows-msvc` |
| `web` | `rapidr-vm-host-web` (bundles, the web IDE) + `rapidr-runtime-web` with `kernel` (native web builds) | `wasm32-unknown-unknown` |
| `tools-<os>` | `rapidr-cli` + `rapidr-launcher` (RapidR's own programs) | every arch of that OS |

For each kind it runs `cargo tree -e normal --target <triple>` (normal
dependencies: what is compiled into the output; build scripts and dev
dependencies are not) and `cargo metadata`, reads each crate's licence files
(`LICENSE*`, `COPYING*`, `COPYRIGHT*`, `NOTICE*`, and those of code bundled
in subfolders, e.g. FreeType's or SDL_GameControllerDB's in their crates),
picks among `A OR B` alternatives the one whose text the crate ships with the
fewest obligations (MIT first), and keeps every `NOTICE` file (Apache-2.0
§4(d); none of today's crates has one). A crate that ships no file for its
licence gets the standard text with its authors as the copyright holders. A
licence outside deny.toml's allowlist is an error: nothing is written.
Components that aren't crates are added by what the graph contains (§3).

**Offline in an install.** A checkout generates the text (cached in
`target/notices/`, keyed by `Cargo.lock`); an install ships it, made at
release time by the same code (`tools/release/stage.py`):
`lib/rapidr/notices/<os>-<arch>.txt` for each runner it ships, `web.txt`, and
`share/doc/rapidr/THIRD-PARTY-NOTICES.txt` for RapidR itself. So
interpreted executables and web bundles get their notices with no Rust and no
network. `tools/build_web_artifacts.sh` writes `target/web/THIRD-PARTY-NOTICES.txt`
next to the web runtime, where the web IDE takes it for its bundles (it
refuses to build a bundle without it).

**No change to programs.** The notices are a file beside the output, not a
command-line switch inside it: a RapidQ program sees exactly the
`COMMAND$` / `ParamStr$` it always did.

## 2. What each output contains

Counts are crates.io packages in the graph today (they change with
`Cargo.lock`; the generated file is always the current truth). "Used under"
is the licence the notices comply with where a crate offers a choice.

### 2.1 Native and interpreted desktop executables

Both contain RapidR's runtime (MIT) and, for the interpreted one, RapidR's
bytecode VM (MIT); the runner stub is built from `rapidr-vm-host-native`,
which is built on `rapidr-runtime-core`, so the two graphs are nearly the
same and one file covers both.

| | macOS (aarch64, x86_64) | Windows (x86_64, aarch64) | Linux (x86_64, aarch64) |
|---|---|---|---|
| Crates | 355 | 350 | 426 |
| Used under MIT | ~305 | ~305 | ~369 |
| Unicode-3.0 (ICU4X data, `unicode-ident`, regex tables) | 23 | 23 | 23 |
| Apache-2.0 (no MIT option shipped) | 14 | 15 | 19 |
| BSD-3-Clause | `encoding_rs` (WHATWG data), `ogg`, `subtle`, `tiny-skia(-path)` | same, without `subtle` | same as macOS |
| BSD-2-Clause | `arrayref`, `serial2` | same | same |
| ISC | `ring`, `rustls-webpki`, `untrusted`, `libloading` | `libloading` | as macOS |
| Zlib | `foldhash` | same | same |
| BSL-1.0 | `ryu`, `xxhash-rust` | + `clipboard-win`, `error-code` | as macOS |
| CDLA-Permissive-2.0 (data) | `webpki-roots` (Mozilla's CA list) | — (Windows uses the system's certificates) | `webpki-roots` |
| MPL-2.0 | `symphonia*` (MP3 decoding, through `rodio`), `option-ext` | + `dwrote` | as macOS |
| Not crates | Rust std (MIT); Liberation fonts (OFL-1.1); SQLite (public domain) | + the toolchain's start-up code (§3.3) | + FreeType (FTL; usually the system's) |
| System, not shipped | libSystem, AppKit, Metal, Core Audio, … | kernel32, user32, UCRT, … | glibc, libgcc_s, libasound, fontconfig, FreeType, X11, Wayland, xkbcommon |

`ring` (TLS for HTTPS through `rustls`, macOS and Linux) contains C and
assembly sourced from BoringSSL (Apache-2.0 or ISC) and its own code (ISC);
its licence files (`LICENSE`, `LICENSE-BoringSSL`, `LICENSE-other-bits`, and
those of the code it bundles) are reproduced in the notices as shipped.

### 2.2 Bytecode (`.rrbc`)

Only the program: its bytecode, its constants and the `$RESOURCE` files it
embeds (the user's own). No third-party code. The RapidR Runtime that runs
it is installed by whoever runs it and carries its own notices (§1).

### 2.3 Web builds

The bundle's files: `index.html`, `loader.js`, `bundle_console.js`,
`ansi_screen.js` (RapidR's, MIT), `rapidrintr.js` (generated by
`wasm-bindgen`, MIT/Apache-2.0), `rapidrintr_bg.wasm` (RapidR's runtime and
compiler and the crates below), the program's `.rrbc`, assets, and
`THIRD-PARTY-NOTICES.txt`.

| | Web (`wasm32-unknown-unknown`) |
|---|---|
| Crates | 109 |
| Used under MIT | 86 |
| Unicode-3.0 | 23 (ICU4X data for text layout) |
| BSD-3-Clause | `tiny-skia`, `tiny-skia-path` |
| BSD-2-Clause | `arrayref` |
| Zlib | `foldhash` |
| Apache-2.0 | `siphasher` |
| Not crates | Rust std (MIT); Liberation fonts (OFL-1.1); SQLite (public domain), musl libc functions and `printf` (MIT) compiled in by `sqlite-wasm-rs` |
| Not shipped | the browser; audio and video files are decoded by the browser (`<audio>`, `<video>`), not by RapidR |

No GPL, LGPL, AGPL or MPL code is in the web runtime. The runtime itself
loads nothing from third-party servers (a program can, of course, with
`RHttp` and the like).

### 2.4 RapidR itself (the SDK and the Runtime)

`rapidr` contains the compiler, the VM and the whole desktop runtime
(`tools-<os>`: 356 crates on macOS and Windows, 427 on Linux); `rapidrw` the
desktop launcher. The SDK also ships the runners and web runtime (with their
notices, §1), the runtime's Rust sources and its crates.io dependencies'
sources (`vendor/`: each crate with its own licence files; crates only other
platforms compile are cut down to their `Cargo.toml` and licence files), and
the web IDE adds Monaco (LICENSES.md §1).

## 3. Components that aren't crates

### 3.1 Fonts

- **Liberation Sans, Serif, Mono 2.1.5** (`crates/rapidr-value/fonts/`):
  SIL Open Font License 1.1, unmodified, compiled into every runtime.
  Reserved Font Names: "Liberation" (Red Hat), "Arimo", "Tinos", "Cousine"
  (Google). The OFL allows bundling with any software, commercial included;
  asks that the copyright notice and licence go with the fonts (the notices
  file carries `OFL-1.1.txt`, whose header has them); forbids selling the
  fonts by themselves; and lets a Modified Version not use a Reserved Font
  Name. RapidR doesn't modify or subset them, so they keep their names.
- **Noto fallback fonts** (`fonts/fallback/`, Stage W7): Noto Sans 2.015,
  Noto Sans Symbols 2.003 and Symbols 2 2.008 (in the repository,
  unmodified), Noto Sans SC and KR from Noto CJK Sans 2.004 (fetched at build
  time, pinned by SHA-256). All are OFL-1.1.
  - **No Reserved Font Name.** None of these fonts declares one: their
    `OFL.txt` / `LICENSE` have no Reserved Font Name line, and the CJK fonts'
    copyright string is "© 2014-2021 Adobe".
  - **Subsets.** `tools/fonts.py` splits them by Unicode range into chunks
    shipped beside the web runtime (`fonts/`); the CJK chunks are renamed
    `<family> NNN`. Both are allowed by the OFL; the copyright lines stay in
    the fonts and in `fonts/fallback/OFL.txt`.
  - **Credits.** That `OFL.txt` goes with the chunks, and the web notices
    carry it (`notices.rs`, `extras()`).

### 3.2 Data built into crates

| Data | Crate | Licence | In the notices |
|---|---|---|---|
| SDL_GameControllerDB (gamepad mappings) | `gilrs` (Windows, macOS) | zlib, © Sam Lantinga | its `LICENSE`, from the crate's subfolder |
| Mozilla's CA certificates | `webpki-roots` (macOS, Linux) | CDLA-Permissive-2.0 (the licence must accompany the data: it does) | yes |
| Unicode data (ICU4X, `unicode-*`, regex tables) | several | Unicode-3.0 (the notice must accompany copies: it does) | yes |
| IANA time zone database | `chrono-tz` | public domain | its `tz/LICENSE` |
| WHATWG encodings index | `encoding_rs` | BSD-3-Clause | yes |
| SQLite | `libsqlite3-sys` (desktop), `sqlite-wasm-rs` (web) | public domain (no notice required; the blessing is reproduced) | yes |
| KDE plasma Wayland protocol descriptions | `wayland-protocols-plasma` (Linux, through winit) | the descriptions: MIT / MIT-CMU / BSD-3-Clause / LGPL-2.1-or-later | a note on the crate; see open question 1 |

### 3.3 The toolchain's and the system's code

| Platform | Linked into the program | Licence | What it asks |
|---|---|---|---|
| All | Rust's standard library (`core`, `alloc`, `std` and the crates they are built from, incl. `compiler_builtins` with code from LLVM compiler-rt) | MIT OR Apache-2.0 (compiler-rt parts: Apache-2.0 WITH LLVM-exception) | notice: in the file (MIT). The LLVM exception waives notices for code embedded in object form |
| Windows, `gnullvm` (LLVM-MinGW) | compiler-rt builtins, libunwind; mingw-w64's start-up objects and helper library | Apache-2.0 WITH LLVM-exception; mingw-w64 runtime licence (`COPYING.MinGW-w64-runtime.txt`) | LLVM parts: nothing. mingw-w64: see open question 2 |
| Windows, `msvc` | Microsoft's C runtime (static with `+crt-static`, as the release does) | Visual Studio licence, "Distributable Code" | no notice; the developer needs a valid Build Tools / Visual Studio licence |
| Windows | kernel32, user32, the Universal CRT, … (DLLs) | part of Windows | nothing: not shipped |
| macOS | libSystem and system frameworks | part of macOS | nothing: not shipped |
| Linux | glibc (`libc.so.6`, `libm`), `libgcc_s` (dynamic) | LGPL-2.1-or-later; GPL-3.0 with the GCC Runtime Library Exception | nothing: dynamically linked system libraries, not shipped |
| Linux | glibc's `crt1.o`/`crti.o`, GCC's `crtbegin.o` (static start-up files) | LGPL with glibc's linking exception; GCC Runtime Library Exception | nothing (the exceptions are for exactly this) |
| Linux | `libasound` (ALSA), fontconfig, FreeType (when the system has it), X11, Wayland, xkbcommon | LGPL-2.1 (alsa-lib); MIT-style; FTL/GPL-2; MIT | nothing: dynamically linked or loaded from the system, not shipped. FreeType is credited anyway, in case `freetype-sys` built its own copy |

## 4. What each licence asks of someone who ships a program

| Licence | Binary or web distribution must… | Done by the notices file? |
|---|---|---|
| MIT, ISC, BSD-2-Clause, Zlib (binary), BSL-1.0 (binary: no requirement), Unicode-3.0, CDLA-Permissive-2.0 | include the copyright notice and licence text (Zlib and BSL-1.0 don't even require it for binaries; included anyway) | yes |
| BSD-3-Clause | the above, and not use the authors' names to endorse the product | yes (and don't) |
| 0BSD, Unlicense, public domain | nothing | listed anyway |
| Apache-2.0 | include the licence; keep copyright, patent and attribution notices; include the `NOTICE` file's attributions if there is one; mark modified files (§4(a)–(d)). Grants a patent licence; it ends for whoever sues over patents in the work | yes (RapidR modifies no Apache-2.0 crate) |
| Apache-2.0 WITH LLVM-exception | for code embedded in object form by compiling, nothing (§4(a), (b), (d) waived) | listed anyway |
| MPL-2.0 (`symphonia*`, `dwrote`, `option-ext`) | file-level copyleft: the rest of the program may be closed. When distributing the executable, the MPL files' source must be available and recipients told how to get it (§3.2); modified MPL files must be published under the MPL | yes: each MPL crate's entry says where its unmodified source is (crates.io) |
| OFL-1.1 (fonts) | ship the copyright notice and licence with the fonts; don't sell the fonts by themselves; a modified font can't use a Reserved Font Name | yes (unmodified) |
| FreeType License (FTL) | credit in the documentation: "Portions of this software are copyright © The FreeType Project (www.freetype.org). All rights reserved." | yes (Linux) |
| GPL, LGPL, AGPL, SSPL, BUSL, non-commercial terms | **not allowed** in anything compiled into a program (deny.toml); only system libraries the user's OS provides, used dynamically, are LGPL (Linux) | — |

## 5. Codecs and patents

- **RapidR's own decoders**, written from the formats' public descriptions
  (`crates/rapidr-value/src/objects/avi.rs`, `codec.rs`, `midifile.rs`,
  `synth.rs`): AVI/RIFF, uncompressed DIB, RLE8/RLE4, Microsoft Video 1
  (CRAM), Cinepak; BMP, ICO; Standard MIDI files and a procedural General
  MIDI synthesizer (no SoundFont or samples); PCM WAV. These formats date
  from 1983–1992; patents last at most 20 years, so any that applied to them
  have expired.
- **Through crates:** JPEG and Motion JPEG (`jpeg-decoder`; baseline JPEG's
  patents have expired), PNG and GIF (`png`, `gif`, `image`: royalty-free;
  LZW's patents expired in 2003–2004), SVG (`resvg`), WAV (`hound`), Ogg
  Vorbis (`lewton`) and FLAC (`claxon`) (royalty-free formats), and
  **MP3 decoding** (`symphonia-bundle-mp3`, through `rodio`'s default
  features: `PLAYSOUND`/`PLAYWAV` can play `.mp3` files on the desktop). MP3's
  patents have expired (the last in 2017) and the licensing programme
  ended.
- **Not present**, decoding or encoding: H.264/AVC, HEVC, MPEG-2/4 video,
  AAC, AC-3, WMA/WMV, any MP3 *encoder*. On the web, audio and video are
  played by the browser's own decoders.

## 6. The repository

Audited for this page (October 2026); the findings and what was done:

- **RapidQ's distribution** (William Yu's freeware; not open source): no file
  of it is in the repository. `.reference/` (the online manual mirror, the
  compiler's messages) is gitignored and untracked. The tests compare
  against RC.EXE's *output* (`tests/rapidq_golden/*.expected`), which is
  program output, not RapidQ's code.
- **Compatibility libraries** (`crates/rapidr-preprocessor/src/libraries/`):
  `QDirListView.inc` and `QDockForm.inc` had method bodies that followed
  user-contributed RapidQ libraries (Rene Saarsoo's QDirListView, Ben Laws'
  and JohnK's QDOCKFORM in RAPIDQ2.INC) too closely; they were rewritten as
  RapidR's own code with the same interface. QDOCKFORM's painting, which
  must look the same for compatibility, was restructured into RapidR's own
  routines (frame, head, grip, edges) and checked to give byte-identical
  window captures for every style, docked, floated and locked. What still
  matches the originals line for line is interface and fact: member names,
  default values, pixel coordinates. The RAPIDQ.INC / qcgi.inc names
  the preprocessor knows are constants' names and values (facts needed for
  compatibility). `qcgi.inc` (GPL) is not included; RapidR's CGI support
  (`objects/cgi.rs`) is its own implementation of the documented behaviour.
- **Tests**: four conformance cases and one GUI fixture that reused the
  manual's example programs nearly word for word were replaced by our own
  programs covering the same features. Short phrases quoted from the manual
  in comments and docs (one sentence each), and about nine of RC.EXE's error
  messages reproduced for compatibility, are kept: short factual quotes.
- **Vendored JavaScript**: Monaco Editor 0.52.2 (MIT, Microsoft), which
  bundles DOMPurify (Apache-2.0 or MPL-2.0), marked (MIT) and the codicon
  icon font (CC-BY-4.0, Microsoft): credited in LICENSES.md. Only the web
  IDE ships it, never a user's program.
- **Fonts named by the web IDE**: its font picker lists Inter, Roboto,
  Montserrat, Nunito, Playfair Display and Fira Code among the families a
  program may ask for. They are names only: no web-font service is
  contacted, and the IDE's own pages use the system's font stack (open
  question 3, settled).
- **Media fixtures**: every `.avi`, `.mid`, `.wav`, `.dxg` and `.bmp` under
  `tests/` is regenerated byte for byte by the repository's scripts
  (`tools/make_avi_fixtures.py`, `make_media_fixture.py`, `make_dx_fixture.py`);
  the remaining four tiny images (`rr_star.svg`, `rr_disc.ico`,
  `picture_res.bmp`, `two_colors.jpg`) are the project's own. No icon is
  taken from RapidQ or an icon pack; the apps use the system's default icon.
- **Names**: "RapidQ", "Visual Basic", "Delphi", "Windows" and others are
  used nominatively ("compatible with", "as in"); LEGAL.md has the
  trademark and no-affiliation statement.

## 7. Guards

- `deny.toml`: the licence allowlist; `cargo deny check licenses` fails on
  anything else (any crate, any target, build and dev dependencies too).
- `crates/rapidr-cli/src/notices.rs`: generation fails on a licence outside
  the same allowlist (a unit test keeps the two lists equal) and on a crate
  that declares none.
- `tools/check_notices.py`: builds a program as an interpreted executable, a
  web bundle and a `.rrbc` (with `--native` and `--web`, natively too),
  checks each output carries its notices and lists every crate `cargo tree`
  finds for that kind — computed independently — and does the same for every
  kind `rapidr notices` makes.
- `python3 tools/third_party_notices.py --check`: the repository-wide list
  (`THIRD_PARTY_NOTICES.md`) is current.
- `tools/regress.sh legal` runs all of them (and is part of a full run).

## 8. Open questions (for a professional review)

1. **KDE plasma Wayland protocols (Linux only).** winit compiles Rust bindings
   generated from KDE's protocol descriptions (`wayland-protocols-plasma`,
   MIT as a crate); the description it uses (`blur.xml`) is
   LGPL-2.1-or-later. The generated code is interface definitions (names,
   opcodes, signatures). The project's reading is that this places no LGPL
   obligation on programs, as with header-file interface data, but it is a
   point for review.
2. **mingw-w64 runtime (Windows, `gnullvm`).** Programs linked with
   LLVM-MinGW contain mingw-w64's start-up code and helper library. Its
   licence (`COPYING.MinGW-w64-runtime.txt`) is mostly ZPL-2.1 / public
   domain / MIT-style; ZPL-2.1 asks for its notice in binary distributions.
   When the release adopts gnullvm, add that file's text to the Windows
   notices (`notices.rs`, `extras()`) after checking which of its parts the
   linked objects come from.
3. **Google Fonts in the web IDE (settled).** Loading fonts from Google's
   servers sent the visitor's IP address to Google, which some EU courts
   have found a GDPR issue without consent. The links are gone: the IDE
   uses the system's fonts, and a program's text is drawn with RapidR's own
   (Liberation, the Noto fallback chunks). Neither the IDE, the runtime nor
   a bundle makes a third-party request a program doesn't make itself.
4. **Encryption export rules.** Desktop programs contain TLS (rustls,
   ring). Publicly available open-source encryption is generally exempt or
   eligible for simple notification under US EAR and similar regimes, but
   this is jurisdiction-specific.
5. **QDockForm / QDirListView history.** The earlier versions were close
   ports of user-contributed RapidQ code whose authors allowed free
   distribution and modification (QDockForm) or stated no terms
   (QDirListView). Both are rewritten now; the old versions remain in git
   history.

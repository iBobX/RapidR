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
the licence it is used under when there is a choice, upstream link, and
notes), and every licence text and notice in full, each identical text once,
with the components it belongs to.

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
in subfolders, e.g. SDL_GameControllerDB's in `gilrs`), picks among
`A OR B` alternatives the one whose text the crate ships with the fewest
obligations (MIT first), and keeps every `NOTICE` file (Apache-2.0 §4(d);
none of today's crates has one). A crate that ships no file for its licence
gets the standard text with its authors as the copyright holders.
Components that aren't crates are added by what the graph contains (§3): the
fonts, SQLite, and the notices of data and protocol descriptions compiled
into crates (read from the crates themselves: the Wayland protocol files'
`<copyright>` blocks, the Adobe Glyph List's header).

**Nothing is written** when a kind's graph has a crate whose licence is not
one of the permissive list (§4: no copyleft of any strength, no data
licences), one of the crates RapidR replaced (§7: `BANNED`), a Wayland
protocol description under anything but MIT or HPND-sell-variant, or a
non-crate component outside its own short list (fonts under the OFL, public
domain, MIT/X11/HPND-style and BSD-3-Clause notices).

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
| Crates | 340 | 338 | 407 |
| Used under MIT | ~290 | ~285 | ~348 |
| Unicode-3.0 (ICU4X data, `unicode-ident`) | 23 | 23 | 23 |
| Apache-2.0 (no MIT option shipped) | 12 (`winit`, `accesskit_winit`, `cpal`, `hound`, `claxon`, `ab_glyph`, …) | 14 | 15 (+ `openssl`, the binding to the system's OpenSSL) |
| BSD-3-Clause | `ogg`, `tiny-skia`, `tiny-skia-path` | same | same |
| BSD-2-Clause | `arrayref`, `serial2` | same | same |
| ISC | `libloading` | same | same |
| Zlib | `foldhash` | same | same |
| BSL-1.0 | `ryu`, `xxhash-rust` | + `clipboard-win`, `error-code` | as macOS |
| Not crates | Rust std (MIT); Liberation fonts (OFL-1.1); SQLite (public domain); the Adobe Glyph List (BSD-3-Clause, in `read-fonts`) | + the toolchain's start-up code (§3.3) | + Cantarell (OFL-1.1, Wayland title bars); the Wayland protocol descriptions (MIT, HPND-sell-variant) and xcb-proto's (X11), from which crates' code is generated |
| System, not shipped | libSystem, AppKit, Metal, Core Audio, Security (HTTPS), … | kernel32, user32, UCRT, SChannel (HTTPS), … | glibc, libgcc_s, OpenSSL 3 (`libssl.so.3`, HTTPS), libasound, fontconfig, X11, Wayland, xkbcommon |

Nothing copyleft (GPL, LGPL, MPL, …), no cryptographic code and no data
under a data licence is compiled in: HTTPS (RHTTP, QDOWNLOAD) uses the
operating system's TLS and certificates through `native-tls`, MP3 is decoded
by `nanomp3`, charts draw their text with the built-in Liberation Sans
(`ab_glyph`), and winit's KDE blur bindings are RapidR's own stand-in (§3.2).
**Linux programs need OpenSSL 3's library at run time** (`libssl.so.3`:
package `libssl3`, `libssl3t64` on Ubuntu 24.04 — part of every current
distribution's base system); building them needs `libssl-dev`
(docs/release-packaging.md).

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
| Used under MIT | 81 |
| Unicode-3.0 | 23 (ICU4X data for text layout) |
| BSD-3-Clause | `tiny-skia`, `tiny-skia-path` |
| BSD-2-Clause | `arrayref` |
| Zlib | `foldhash` |
| Apache-2.0 | `siphasher` |
| Not crates | Rust std (MIT); Liberation fonts (OFL-1.1); SQLite (public domain), musl libc functions and `printf` (MIT) compiled in by `sqlite-wasm-rs`; the Adobe Glyph List (BSD-3-Clause, in `read-fonts`) |
| Not shipped | the browser; audio and video files are decoded by the browser (`<audio>`, `<video>`), not by RapidR |

No GPL, LGPL, AGPL or MPL code, and no cryptography, is in the web runtime
(HTTPS is the browser's). The runtime itself
loads nothing from third-party servers (a program can, of course, with
`RHttp` and the like).

### 2.4 RapidR itself (the SDK and the Runtime)

`rapidr` contains the compiler, the VM and the whole desktop runtime
(`tools-<os>`: 342 crates on macOS, 341 on Windows, 408 on Linux; the same
permissive list, §4); `rapidrw` the
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
- **Noto fallback fonts (planned)**: OFL-1.1 as well. Subsetting a font into
  chunks makes a Modified Version under the OFL. Before adding them: check
  each font's `OFL.txt` for a Reserved Font Name (Noto fonts are published
  without one, but check the exact files); either way, the simplest always-
  compliant path is to give the chunks RapidR's own family names (e.g.
  "RapidR Fallback 1"), keep the original copyright lines, and add their
  `OFL.txt` to the generator's extras (`notices.rs`, `extras()`), like
  Liberation's.
- **Cantarell Regular** (Linux only): built into the `sctk-adwaita` crate,
  through which winit draws a window's title on Wayland desktops that leave
  the title bar to the program (GNOME); unmodified; OFL-1.1, © 2009-2011
  Understanding Limited, © 2010-2011 Jakub Steiner. The crate ships no text
  for it: the generator adds the font's copyright lines and the OFL.

### 3.2 Data built into crates

| Data | Crate | Licence | In the notices |
|---|---|---|---|
| SDL_GameControllerDB (gamepad mappings) | `gilrs` (Windows, macOS) | zlib, © Sam Lantinga | its `LICENSE`, from the crate's subfolder |
| Unicode data (ICU4X, `unicode-*`, HarfRust's tables) | several | Unicode-3.0 (the notice must accompany copies: it does) | yes |
| IANA time zone database | `chrono-tz` | public domain | its `tz/LICENSE` |
| SQLite | `libsqlite3-sys` (desktop), `sqlite-wasm-rs` (web) | public domain (no notice required; the blessing is reproduced) | yes |
| Adobe Glyph List (glyph names → characters) | `read-fonts` (through `parley`; all outputs) | BSD-3-Clause, © Adobe | the list's own notice, read from the crate |
| Wayland protocol descriptions: `wayland.xml`; wayland-protocols' (xdg-shell, viewporter, fractional-scale, text-input, pointer-gestures, …); wlr-protocols' | `wayland-client`, `wayland-protocols`, `wayland-protocols-wlr` (Linux, through winit and smithay-client-toolkit): their Rust code is generated from these XML files | each file's own notice: MIT (most), HPND-sell-variant (text-input-v3, pointer-gestures, ext-data-control, ext-foreign-toplevel-list, ext-workspace; six of wlr's) — both permissive: a notice, and no endorsement | every file's notice, read from the crates (generation fails on any other licence) |
| xcb-proto (the X11 protocol descriptions) | `x11rb-protocol` (Linux), generated from them | X11 (MIT with a no-advertising clause), © Bart Massey, Jamey Sharp, Josh Triplett | yes (the crate doesn't ship the text: the generator carries it) |
| AT-SPI's D-Bus interfaces | `atspi-proxies` (Linux screen readers, through AccessKit), generated from at-spi2-core's introspection files (LGPL-2.1-or-later) | the crates: MIT OR Apache-2.0. What is compiled in is the interfaces' names and signatures — what any program talking to the accessibility bus over D-Bus must send; the XML files themselves are only read by the crate's tests | §8, question 1 |
| KDE's Wayland protocols | **none**: crates.io's `wayland-protocols-plasma` (generated from KDE's protocol files, `blur.xml` and others LGPL-2.1-or-later) is replaced in every build by RapidR's stand-in, `crates/patches/wayland-protocols-plasma` (MIT): the Rust names winit uses, generated from RapidR's own protocol file whose interfaces no compositor announces, so winit finds no blur manager and never blurs (RapidR never asks for blur) | — | — (the workspace's `[patch.crates-io]`, every generated program's `Cargo.toml` — codegen's `PATCHES` — and an install's home, `tools/release/home.py`, carry it) |

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
| Linux | OpenSSL 3 (`libssl.so.3`, `libcrypto.so.3`: HTTPS) | Apache-2.0 | nothing: dynamically linked from the system, not shipped (a program needs the system's `libssl3`) |
| Linux | `libasound` (ALSA), fontconfig, X11, Wayland, xkbcommon | LGPL-2.1 (alsa-lib); MIT-style; MIT | nothing: dynamically linked or loaded from the system, not shipped |
| macOS, Windows | Security.framework; SChannel (HTTPS) | part of the system | nothing |

## 4. What each licence asks of someone who ships a program

| Licence | Binary or web distribution must… | Done by the notices file? |
|---|---|---|
| MIT, ISC, BSD-2-Clause, Zlib (binary), BSL-1.0 (binary: no requirement), Unicode-3.0 | include the copyright notice and licence text (Zlib and BSL-1.0 don't even require it for binaries; included anyway) | yes |
| HPND-sell-variant, X11 (protocol descriptions, §3.2) | the copyright and permission notice in the documentation; don't use the authors' names to promote the product | yes (and don't) |
| BSD-3-Clause | the above, and not use the authors' names to endorse the product | yes (and don't) |
| 0BSD, Unlicense, CC0-1.0, public domain | nothing | listed anyway |
| Apache-2.0 | include the licence; keep copyright, patent and attribution notices; include the `NOTICE` file's attributions if there is one; mark modified files (§4(a)–(d)). Grants a patent licence; it ends for whoever sues over patents in the work | yes (RapidR modifies no Apache-2.0 crate) |
| Apache-2.0 WITH LLVM-exception | for code embedded in object form by compiling, nothing (§4(a), (b), (d) waived) | listed anyway |
| OFL-1.1 (fonts) | ship the copyright notice and licence with the fonts; don't sell the fonts by themselves; a modified font can't use a Reserved Font Name | yes (unmodified) |
| GPL, LGPL, AGPL, MPL, EPL, CDDL or any other copyleft; SSPL, BUSL, non-commercial terms; data licences (CDLA, …); advertising clauses (BSD-4-Clause, OpenSSL-1.x / SSLeay); FreeType's FTL | **not allowed** in anything compiled into a program (§7); only system libraries the user's OS provides, used dynamically, are LGPL (Linux's glibc and libasound) | — |

So the permissive list — the only licences a crate compiled into a program
may be used under — is: MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception,
BSD-2-Clause, BSD-3-Clause, ISC, Zlib, 0BSD, BSL-1.0, Unlicense, Unicode-3.0
and CC0-1.0; plus OFL-1.1 for fonts. All any of them asks of a user is that
`THIRD-PARTY-NOTICES.txt` go with the program.

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
  **MP3 decoding** (`nanomp3`, MIT OR Apache-2.0: a pure-Rust port of the
  public-domain minimp3; `PLAYSOUND`/`PLAYWAV` can play `.mp3` files on the
  desktop). MP3's patents have expired (the last in 2017) and the licensing
  programme ended.
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
- **Fonts loaded by the web IDE**: `web-ide/index.html` and `preview.html`
  load Inter, Roboto, Montserrat, Nunito, Playfair Display and Fira Code
  from Google Fonts (OFL / Apache-2.0; served by Google, not redistributed).
  See open question 3.
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

- `deny.toml`: the permissive list (§4) is its allowlist, for the whole
  workspace — every crate, every target, RapidR's own tools and build and
  dev dependencies too: `cargo deny check licenses` fails on anything else.
  Its `[bans]` deny the crates RapidR replaced (`ring`, `rustls`,
  `aws-lc-*`, `openssl-src`, `webpki-roots`, `symphonia*`, `font-kit`,
  `dwrote`, `option-ext`, `freetype-sys`): `cargo deny check bans` fails if
  one comes back.
- `crates/rapidr-cli/src/notices.rs`: for each kind of output (desktop per
  target, web, tools), generation fails on a crate outside the permissive
  list (`ALLOWED`; unit tests keep it equal to the list and to deny.toml's),
  on a `BANNED` crate (the same, and crates.io's `wayland-protocols-plasma`),
  on a crate that declares no licence, on a Wayland protocol file under
  anything but MIT or HPND-sell-variant, and on a non-crate component
  outside `EXTRA_ALLOWED`.
- `tools/check_notices.py`: on its own (its list, its `cargo tree` per kind,
  its SPDX evaluation), checks that every kind's graph keeps to the
  permissive list and has none of the replaced crates nor crates.io's KDE
  bindings, and that deny.toml's allowlist is the list; builds a program as
  an interpreted executable, a web bundle and a `.rrbc` (with `--native` and
  `--web`, natively too), checks each output carries its notices and lists
  every crate in it, and does the same for every kind `rapidr notices` makes.
- `crates/rapidr-codegen-rust`: a unit test keeps a generated program's
  `[patch.crates-io]` equal to the workspace's.
- `python3 tools/third_party_notices.py --check`: the repository-wide list
  (`THIRD_PARTY_NOTICES.md`) is current.
- `tools/regress.sh legal` runs all of them and exits 1 if any fails (it is
  part of a full run).

## 8. Open questions (for a professional review)

Resolved by removing what raised them (October 2026):

- **KDE's plasma Wayland protocols** (was question 1). winit's bindings to
  KDE's blur protocol, generated from LGPL-2.1-or-later protocol files, are
  no longer compiled into anything: RapidR's own stand-in replaces the crate
  (§3.2), and the guards fail if crates.io's comes back (§7).
- **Cryptography in programs** (was question 4, export rules). No program
  contains a cryptographic library any more: HTTPS is the operating
  system's (Security.framework, SChannel, the system's OpenSSL 3), and
  `ring`, `rustls`, `rustls-webpki` and `webpki-roots` (CDLA data) are gone.
  A program that calls the system's TLS is generally outside encryption
  export controls' "contains encryption" cases, but that remains a question
  of law, not of licences, and of the user's own jurisdiction.
- **MPL-2.0** (`symphonia*`, `dwrote`, `option-ext`): removed; MP3 is
  `nanomp3`'s, charts no longer use font-kit, the CLI no longer uses `dirs`.
  With them went FreeType (its FTL asks for credit in the documentation) and
  fontconfig's link for charts.

Still open:

1. **AT-SPI interface names (Linux).** AccessKit's Linux adapter talks to
   the accessibility bus with `atspi-proxies`, whose Rust code was generated
   (zbus-xmlgen) from at-spi2-core's D-Bus introspection files, which are
   LGPL-2.1-or-later. What is compiled in is the interfaces' names, method
   names and type signatures — what any program speaking to the bus must
   send — and the program and the bus are separate processes. The project's
   reading: interface facts needed for interoperability, used over IPC,
   carry no LGPL obligation (the LGPL itself treats such header-level
   material as unrestricted, §5 of 2.1). Removing it would remove screen
   readers on Linux; there is no other licence for the interface. For
   review.
2. **mingw-w64 runtime (Windows, `gnullvm`).** Programs linked with
   LLVM-MinGW contain mingw-w64's start-up code and helper library. Its
   licence (`COPYING.MinGW-w64-runtime.txt`) is mostly ZPL-2.1 / public
   domain / MIT-style; ZPL-2.1 asks for its notice in binary distributions.
   When the release adopts gnullvm, add that file's text to the Windows
   notices (`notices.rs`, `extras()`) after checking which of its parts the
   linked objects come from. (With `msvc`, Microsoft's runtime is
   redistributable code under the Visual Studio licence: no notice.)
3. **Google Fonts in the web IDE.** Loading fonts from Google's servers
   sends the visitor's IP address to Google; some EU courts have found that
   a GDPR issue without consent. It concerns whoever hosts the IDE, not
   users' programs. Options: self-host the fonts (their licences allow it)
   or use system fonts.
4. **QDockForm / QDirListView history.** The earlier versions were close
   ports of user-contributed RapidQ code whose authors allowed free
   distribution and modification (QDockForm) or stated no terms
   (QDirListView). Both are rewritten now; the old versions remain in git
   history.

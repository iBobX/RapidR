# Release packaging

How a RapidR release is built — on this Mac and in the Parallels VMs, never by
remote CI — and how to publish it on GitHub. The scripts are in
`tools/release/`; nothing in them pushes or uploads.

A release has, for each platform, the **SDK** (the IDE, the compiler, the
runtime, the runtime's sources for native builds) and the **Runtime** alone
(runs programs, like a JRE), plus the web IDE, checksums, an SBOM and the
release notes:

| | SDK | Runtime |
|---|---|---|
| macOS (universal) | `RapidR-<ver>-macos-universal.dmg` (RapidR.app + RapidR Runtime.app) | `RapidR-Runtime-<ver>-macos-universal.dmg` |
| Windows x64, ARM64 | `RapidR-<ver>-windows-<x64\|arm64>-setup.exe` | `RapidR-Runtime-<ver>-windows-<arch>-setup.exe` |
| Linux x86_64, aarch64 | `rapidr-<ver>-linux-<arch>.tar.gz`, `rapidr_<ver>_<amd64\|arm64>.deb` | `rapidr-runtime-…tar.gz`, `rapidr-runtime_…deb` |
| Web | `rapidr-web-<ver>.zip` | |
| All | `SHA256SUMS`, `rapidr-<ver>.cdx.json` (CycloneDX SBOM), `RELEASE_NOTES.md` | |

## What an installed RapidR is

Every package installs the same tree, a *prefix* (`tools/release/stage.py`
lays it out):

```
bin/rapidr[.exe]                 the CLI — and the RapidR Runtime (rapidr run / open)
bin/rapidrw[.exe]                the desktop's launcher (Windows; the macOS apps' executable)
lib/rapidr/                      RapidR's home
  release.toml                   version, the Rust it was tested with, kind = sdk | runtime
  ide/rapidr-ide.rrbc            the IDE (rapidr ide)                              SDK
  runners/<os>-<arch>/           rapidrintr-runner[w][.exe]: --interp executables  SDK
  web/                           rapidrintr.js + .wasm: rapidr bundle-bc           SDK
  Cargo.toml, Cargo.lock,        the runtime crates' sources and their crates.io   SDK
  crates/, vendor/, .cargo/      dependencies, vendored: native builds offline
share/doc/rapidr/                LICENSE, LEGAL.md, LICENSES.md, THIRD_PARTY_NOTICES.md,
                                 THIRD-PARTY-NOTICES.txt (rapidr's own), OFL-1.1.txt, README.md
lib/rapidr/notices/              <os>-<arch>.txt per runner, web.txt: the          SDK
                                 THIRD-PARTY-NOTICES.txt every build writes
```

On macOS `bin/` is `RapidR.app/Contents/MacOS/` and `lib/` is
`Contents/lib/` (codesign accepts it, strict); the `.deb` installs
`/usr/bin/rapidr` and `/usr/lib/rapidr`; Windows `%LOCALAPPDATA%\Programs\RapidR`;
the `.tar.gz`'s `install.sh` `~/.local`.

**How the CLI finds its files** (`crates/rapidr-cli/src/home.rs`) — one rule:
`RAPIDR_HOME` if set; else `<the executable's folder>/../lib/rapidr` when it
has `release.toml` (an install); else the source checkout above the current
directory or the executable (unchanged). An install's home is laid out like a
checkout's root for what builds read, so native and web builds take the same
path; what differs is what a checkout builds itself and an install ships: the
runner stubs (`runners/`, chosen with `--target <os>-<arch>`; a checkout
builds its own with cargo as before), the web interpreter (`web/`) and the
crates.io sources (`vendor/`: cargo runs with `--offline` and a vendored
source replacement, nothing downloaded).

- **No Rust needed** to run programs (`rapidr run`, double clicks), to make
  standalone interpreted executables (`rapidr build x.bas --interp`, from the
  shipped runner) or to use the IDE. Only native builds need Rust.
- **`rapidr setup`** installs it when asked: rustup (MIT / Apache-2.0) into
  `~/.cargo` and `~/.rustup`, after saying so and asking (`--yes` to agree,
  `--check` to only report); it also offers to link `rapidr` into
  `/usr/local/bin` or `~/.local/bin` when it isn't on PATH (the macOS app, a
  `.tar.gz` not installed). The CLI finds cargo on PATH or in rustup's folder,
  so the IDE started from the desktop builds too.
- **The vendored crates** are those the runtime's builds use on that OS and
  for the web: a crate only other platforms compile keeps its `Cargo.toml`
  (resolution reads it) and nothing else (`tools/release/home.py`) — about
  0.37 GB of sources instead of 1.2 GB. Checked: a native build with an empty
  `CARGO_HOME`, offline, from an install's home.

### Windows' linker: the recommendation

Rust on Windows needs a linker. Two ways:

- **`gnullvm` — recommended**: the `*-pc-windows-gnullvm` targets with
  [LLVM-MinGW](https://github.com/mstorsjo/llvm-mingw) (Apache-2.0 with LLVM
  exception; mingw-w64's runtime under ZPL / public-domain / MIT-style terms).
  Fully open source, x64 and ARM64, no Microsoft licence; programs link the
  system's UCRT; nothing it adds to an executable carries obligations. Users
  unzip it and put its `bin\` on PATH (`rapidr setup --toolchain gnullvm`
  says how). Not yet tried in the VM: the first Windows run decides it.
- **`msvc`**: what rustup offers by default — Microsoft's C++ Build Tools,
  under Visual Studio's licence (free for individuals, open source and small
  organisations; larger ones need a Visual Studio licence). Proprietary, so
  not what the release itself is built with if gnullvm works; fine as the
  user's own choice (`rapidr setup --toolchain msvc`). The release scripts
  accept `-Toolchain msvc` (they then link the MSVC runtime statically, so
  users need no VC++ redistributable).

## File types: the RapidR Runtime on the desktop

| | double click | secondary action |
|---|---|---|
| `.rrbc` (a compiled program) | runs (`rapidr open`) | |
| `.rr`, `.bas` (source) | opens in the IDE (SDK; the Runtime alone: runs) | Run (Windows: the Run verb; macOS / Linux: Open With > RapidR Runtime) |

- **Windows** (`tools/release/windows/rapidr.iss`): `HKCU\Software\Classes`
  ProgIDs `RapidR.Program` and `RapidR.Source` with open / run verbs and an
  icon, `OpenWithProgids` for `.rr` / `.bas`; per user, no admin; the
  uninstaller removes them. They go through **`rapidrw.exe`**, a windowed
  launcher beside the console `rapidr.exe` (as `pythonw.exe` beside
  `python.exe`): it asks `rapidr info` the program's `$APPTYPE` and gives a
  console program a new console, a windowed one none. Standalone `--interp`
  executables for Windows are built from the windowed runner
  (`rapidrintr-runnerw.exe`) when the program isn't a console one — no
  console window opens with a GUI program, as RapidQ's.
- **macOS** (`tools/release/macos/*.plist`): RapidR.app exports the `rr` and
  `bas` types (`UTExportedTypeDeclarations`) and edits them
  (`CFBundleDocumentTypes`, Owner); RapidR Runtime.app owns `rrbc` and is the
  Alternate handler of `rr` / `bas`. Finder hands an app the files it opens as
  an Apple event, so both apps' executable is `rapidrw`
  (`crates/rapidr-launcher`, AppKit through objc2 — MIT): it receives them and
  `exec`s `rapidr ide <file>` or `rapidr open <file>`. Launch Services
  registers the apps when they are copied to Applications; no script touches
  it.
- **Linux** (`tools/release/linux/`): shared-mime-info types
  `application/x-rapidr-bytecode` (`*.rrbc`, magic `RRBC`), `text/x-rapidr`,
  `text/x-rapidq-basic`; `rapidr-runtime.desktop` (`rapidr open %f`) and
  `rapidr-ide.desktop` (`rapidr ide %f`) with their MimeType; defaults set with
  `xdg-mime`. The `.deb` puts them in `/usr/share` (dpkg's triggers update the
  databases); the `.tar.gz`'s `install.sh` in `~/.local/share`, and
  `uninstall.sh` removes them and the defaults.

**Console or windowed** is the program's `$APPTYPE` (RapidQ's CONSOLE, GUI,
CGI; RapidR's WEB), recorded in the bytecode. Without one, a program that
creates components is GUI, else CONSOLE — what RapidQ does ("Rapid-Q will
detect what kind of application your program is"). On macOS and Linux a
console program opened from the desktop gets a terminal (Terminal; Linux:
`$TERMINAL`, `x-terminal-emulator`, or a known one), which stays open when it
ends.

**The bytecode header** (format 3; `interpreter/rapidr-bytecode`): after
`RRBC`, the format and the flags, a header that every later format keeps:
its length, the oldest runtime that runs the program (3 × u16) and the app
type. A runtime reads formats 2 (no header) and 3; a program for a newer
runtime is refused with "this program needs RapidR Runtime x.y.z or newer
(this is …); get it from https://github.com/iBobX/RapidR/releases".
`MIN_RUNTIME` is raised whenever the compiler starts writing something older
runtimes don't know (an opcode, a builtin). `rapidr info <file>` prints it all.

**Scripts:** `#!/usr/bin/env rapidr` as a `.rr` / `.bas` file's first line
(the preprocessor leaves that line out) and `chmod +x`: `./script.rr args`
runs it. `rapidr <file.rrbc> [args]` runs a program too. (`rapidr <file.rr>`
without a `#!` line still builds it, as it always has.)

**Downloaded files** ask once before they run: macOS' quarantine attribute,
Windows' Mark of the Web (`Zone.Identifier`, zone 3 or 4), and on Linux the
origin Chromium and Firefox record (`user.xdg.origin.url`). The answer is kept
per file content (SHA-256) in `trusted-files.txt` in the user's RapidR folder
(`~/Library/Application Support/RapidR`, `%APPDATA%\RapidR`,
`~/.config/rapidr`; `RAPIDR_CONFIG_DIR` elsewhere); on a terminal the
question is asked there, else in a window.

**A program run by the runtime** is the program: `Application.ExeName` and
`Application.Path` name its file, `COMMAND$` is its arguments — as if it
were built. `RAPIDR_RUNTIME` names the `rapidr` running it (the IDE builds
with it).

**Standalone executables stay**: `rapidr build x.bas --interp` makes one that
needs no runtime, for programs shipped without it.

## Licences

Everything shipped is open source and allows commercial use; nothing GPL,
LGPL or AGPL is in any artifact; programs built with RapidR are their
authors', to ship as they like.

- Rust crates: checked by `cargo deny check licenses` against `deny.toml`
  (MIT, Apache-2.0, BSD, ISC, zlib, 0BSD, BSL-1.0, Unlicense, Unicode-3.0,
  CDLA-Permissive-2.0, MPL-2.0 for a few unmodified crates), listed in
  `THIRD_PARTY_NOTICES.md` (`python3 tools/third_party_notices.py --check`)
  and in the SBOM.
- **What builds carry:** `stage.py` writes, with `rapidr notices` (the CLI's
  generator, from this checkout's graph), `lib/rapidr/notices/<target>.txt`
  for every runner it stages and `web.txt` (an install copies them beside
  every executable and into every web bundle it builds, offline), and
  `share/doc/rapidr/THIRD-PARTY-NOTICES.txt` for RapidR's own programs.
  `LEGAL.md` is in every package; the Windows installer shows it before
  installing (`InfoBeforeFile`), after the MIT licence page. See
  `docs/licensing.md`. A crate offered under a choice that includes LGPL
  (r-efi: MIT OR Apache-2.0 OR LGPL-2.1+) is used under MIT.
- JS / wasm: Monaco (MIT) in the web IDE; the wasm-bindgen glue (MIT /
  Apache-2.0) — `LICENSES.md`. Fonts: Liberation (OFL-1.1).
- New here: `rapidr-launcher` uses objc2, objc2-foundation, objc2-app-kit
  (MIT, already in the tree through winit); the CLI uses dirs, sha2, libc
  (MIT / Apache-2.0, already in the tree).
- Installer runtime pieces: Inno Setup's setup and uninstaller stubs (Inno
  Setup licence: free for any use including commercial, source available;
  no obligations on what it installs). The macOS disk image and the `.deb`
  carry no installer code; `install.sh` / `uninstall.sh` are RapidR's (MIT).
- System libraries the Linux binaries link (not shipped): glibc, ALSA,
  fontconfig, FreeType, xkbcommon — dynamically, which puts no obligation on
  RapidR's packages.
- Build tools whose output carries no obligations: rustc / cargo (MIT /
  Apache-2.0), LLVM-MinGW (Apache-2.0 + LLVM exception), wasm-pack and
  wasm-bindgen (MIT / Apache-2.0), Python (PSF), Inno Setup, dpkg-deb
  (GPL-2.0 — a copyleft build tool: the packages it writes are not its
  work and carry no obligation), GNU tar and gzip (GPL — same), zip
  (Info-ZIP), Docker (Apache-2.0); Apple's lipo, codesign, hdiutil, plutil
  and Microsoft's signtool and MSVC (if chosen) are the platforms' own,
  proprietary, and add nothing to the packages' licences (MSVC's statically
  linked runtime is Microsoft's redistributable code — another reason for
  gnullvm). Icons: none yet (the system's generic ones) — RapidR's own
  artwork, when made, under its MIT licence.

## Signing and notarization — the user's decision

Unsigned is the default; every script works without a certificate.

- **macOS**: `tools/release/macos.sh --sign "Developer ID Application: Name
  (TEAMID)" --notarize <profile>` signs inside-out with the hardened runtime
  and a timestamp (`rapidr` gets `disable-library-validation`: programs load
  their own `DECLARE … LIB` libraries), signs the disk images, submits them
  with `xcrun notarytool` (a keychain profile made once with `xcrun notarytool
  store-credentials`) and staples the ticket. Needs an Apple Developer ID (Apple
  Developer Program, yearly fee). **Unsigned**: the apps are signed ad hoc;
  downloaded, Gatekeeper refuses the first double click — right-click > Open
  (or System Settings > Privacy & Security > Open Anyway), once.
- **Windows**: `windows.ps1 -Sign "<certificate subject>"` signs the
  executables and installers with signtool and a timestamp. Needs a
  code-signing certificate (OV or EV; EV builds SmartScreen reputation at
  once). **Unsigned**: SmartScreen says "Windows protected your PC" — More
  info > Run anyway.
- **Known limit**: a standalone `--interp` executable is the runner with the
  program appended, which a signature can't cover (macOS refuses to sign
  such a file; on Windows the program must stay the file's last bytes). Users
  who sign what they ship use native builds, or ship the `.rrbc` with the
  RapidR Runtime. (A later change: the payload as a Mach-O section / PE
  resource.)

## Cutting a release, step by step

Disk: about 10 GB free on the Mac for the macOS builds (two architectures)
and the web interpreter; the Linux build uses ~5 GB inside the Ubuntu VM
(or a Docker volume per architecture); the Windows build ~8 GB inside the VM.
`dist/<ver>/out` is about 0.8 GB for the whole release. Every script removes
its `work/` folder when it ends.

1. **The commit.** On `development`: bump the version (`Cargo.toml`),
   `CHANGELOG.md`, tick ROADMAP; `tools/regress.sh` passes; `cargo deny check
   licenses` and `python3 tools/third_party_notices.py --check` pass. Set
   `MIN_RUNTIME` (`interpreter/rapidr-bytecode/src/lib.rs`) to this version if
   the compiler writes anything new for the runtime. Commit; the tree must be
   clean.
2. **Prepare** (Mac): `tools/release/prepare.sh` — `dist/<ver>/prep/`
   (src.tar of the commit, the web interpreter, the IDE's bytecode) and the
   web bundle, the SBOM and `RELEASE_NOTES.md` in `dist/<ver>/out/`. Edit the
   notes' "What's new" from `CHANGELOG.md`.
3. **macOS** (Mac): `rustup target add x86_64-apple-darwin` once, then
   `tools/release/macos.sh` (`--sign …`, `--notarize …` when signing).
   Check: `tools/release/smoke.sh dist/<ver>/out/RapidR-<ver>-macos-universal.dmg`
   and the Runtime's dmg.
4. **Linux aarch64** (Ubuntu VM): `tools/release/linux-vm.sh` builds both
   packages in the VM and fetches them. Check in the VM: send the artifact and
   `tools/release/smoke.sh` (`tools/release/ubuntu-vm.sh send …` / `run …`),
   then `tools/release/linux-vm.sh --clean`.
   **Linux x86_64**: `tools/release/linux-docker.sh x86_64` (Docker Desktop,
   emulated: slow, ~10 GB free on the Mac) — or `linux.sh` on any x86_64
   Ubuntu 24.04 from the extracted src.tar. (`linux-docker.sh aarch64` is the
   Docker way for aarch64.) AppImage: not made — the `.tar.gz` already runs
   from any folder and installs per user, and an AppImage would add FUSE and
   one more runtime to ship; reconsider if users ask.
5. **Windows** (Windows VM; once: Python 3.11+, Inno Setup 6, LLVM-MinGW's
   `bin\` on PATH — or VS Build Tools with the x64 and ARM64 tools for
   `-Toolchain msvc`): `tools/release/windows-vm.sh build` (add
   `-Publish \\Mac\Home\…\dist\<ver>\out` to have the VM copy the installers
   to the Mac, or copy them from `%USERPROFILE%\rapidr-release\out`). Check:
   `tools/release/windows-vm.sh smoke RapidR-<ver>-windows-arm64-setup.exe -Associations`,
   and the x64 one (ARM64 Windows runs it emulated).
6. **Finish** (Mac): `tools/release/finish.sh` — every package has the
   licence files; `SHA256SUMS`.
7. **Publish** — by hand, after looking at `dist/<ver>/out/` (the scripts
   never do this):

   ```bash
   cd dist/<ver>/out
   gh release create v<ver> --repo iBobX/RapidR --target <commit> \
       --title "RapidR <ver>" --notes-file RELEASE_NOTES.md \
       *.dmg *.exe *.tar.gz *.deb rapidr-web-<ver>.zip rapidr-<ver>.cdx.json SHA256SUMS
   ```

   (`--draft` first to look at it on GitHub before it's public; `--prerelease`
   for a preview.)
8. **Clean up**: `rm -rf dist/<ver>/work`, the Docker volumes
   (`docker volume rm rapidr-release-target-<arch> rapidr-linux-registry-<arch>`),
   `tools/release/linux-vm.sh --clean`, the Windows VM's `%USERPROFILE%\rapidr-release`.

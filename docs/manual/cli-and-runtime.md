# The CLI and the RapidR Runtime

One command, `rapidr`, is the compiler, the build tool and the runtime.

## Commands

| Command | |
|---|---|
| `rapidr run <file> [args]` | Run a program: a `.rrbc`, or a `.rr` / `.bas` source (compiled in memory first). This is the RapidR Runtime. |
| `rapidr <file.rrbc> [args]` | The same, for a compiled program (and for `#!/usr/bin/env rapidr` scripts) |
| `rapidr open <file> [args]` | Run it as a double click does: a downloaded file asks first, a console program gets a terminal |
| `rapidr info <file>` | The program's kind, bytecode format and the oldest runtime it needs |
| `rapidr ide [file]` | The IDE (SDK) |
| `rapidr build <file> [outdir] [flags]` | Build an executable or a web build (below) |
| `rapidr <file.rr\|.bas> [flags]` | Shortcut for `build`, optimized by default |
| `rapidr build <app.rrproj>` | Build a project's main file with its app settings |
| `rapidr install-app <Name.AppDir>` | Linux: put a built app in your applications menu |
| `rapidr build-bc <file> [-o out.rrbc]` | Compile to bytecode, for the Runtime |
| `rapidr bundle-bc <file> [-o out.zip]` | A static web bundle (a `.zip`) |
| `rapidr setup [--check] [--yes] [--no-path]` | Install the Rust native builds use; put `rapidr` on PATH |
| `rapidr notices [<os>-<arch>\|web\|tools-<os>] [-o file]` | Print the third-party notices a kind of build carries |
| `rapidr about`, `rapidr version` | |
| `rapidr --log <file> <command…>` | A command's output (and cargo's) in a file |
| `rapidr preprocess`, `lex`, `parse`, `codegen <file> [outdir]` | The compiler's stages, for curiosity and bug reports |

## Builds

```sh
rapidr build prog.bas --interp             # standalone, the interpreter inside
rapidr build prog.bas --interp --target windows-x86_64
rapidr build prog.bas                      # native: a release build, optimized
rapidr build prog.bas --debug              # native, for debugging (quicker to build)
rapidr build prog.bas dist                 # the app in dist/
rapidr build prog.bas --keep-rust          # and the generated Rust in prog-rust-source/
rapidr build prog.bas --web                # native code for the browser
rapidr build prog.bas --web --interp       # web bundle: prog-web.zip
```

| Flag | |
|---|---|
| `--interp`, `-i` | the bytecode interpreter: no Rust needed |
| `--release`, `-r` / `--debug`, `-d` | optimized, what you ship (the default) / quick to compile, for debugging |
| `[output folder]`, `--output <folder>`, `-o <folder>` | where the app goes: else the project's output folder (`build`), else the source's folder |
| `--keep-rust` / `--no-keep-rust` | a native build also leaves the Rust it generated in `<output>/prog-rust-source/` (with a README) / deletes it (the default; a project's `keep_rust` says otherwise) |
| `--web`, `-w` | for the browser (also when the program says `$APPTYPE WEB`) |
| `--icon`, `--name`, `--bundle-id`, `--app-version`, `--company`, `--project`, `--bundle` / `--no-bundle`, `-g<icon>` | the app it becomes and its icon: [Building apps and their icons](building-apps.md) |
| `--target <os>-<arch>` | an interpreted build for another architecture, from the runners the SDK ships: on Windows `windows-x86_64` and `windows-aarch64`; on macOS `macos` (universal, the default), `macos-arm64`, `macos-x86_64`; on Linux the machine's own (`linux-x86_64` or `linux-aarch64`). A source checkout builds the runner it needs with cargo |

Where things go — the output folder: the one you name, else the project's
(`[build] output`, `build` by default) when the program has a `.rrproj`, else
the source's own folder (RapidQ's way). It gets only what you ship:

| Build | Output |
|---|---|
| `--interp` | `prog` (`prog.exe` on Windows) — on macOS universal (Apple silicon and Intel) by default; a program with windows becomes `prog.app` (macOS) or `prog.AppDir` (Linux), and every `.exe` gets its icon ([Building apps](building-apps.md)) |
| native | the same (the generated Rust and cargo's files stay in the build cache: below) |
| `--web` | `prog_web/`: `index.html`, the program's `.wasm` and `.js` |
| `--web --interp`, `bundle-bc` | `prog-web.zip` |

Every build also writes **`THIRD-PARTY-NOTICES.txt`** beside the executable,
inside the app (`Contents/Resources`, the AppDir) or into the web build: the notices and licence texts of everything inside
it, made from the build's real dependency graph. Ship it with the program;
that is all the licences ask ([LEGAL.md](../../LEGAL.md)).

On Windows, a GUI program builds as a windowed executable: no console
window opens with it.

### Native builds

A native build translates the program to Rust and compiles it with cargo:

- Run `rapidr setup` once. An installed SDK always builds with the Rust
  version it was tested with (named to cargo, never made your default) and
  works offline: the runtime's sources and their crates are in the SDK.
- Windows: the SDK ships LLVM-MinGW as linker, so no Visual Studio is
  needed, and programs need no DLL beside them. `RAPIDR_TOOLCHAIN=msvc`
  (with `rapidr setup --toolchain msvc`) uses Microsoft's toolchain
  instead.
- macOS: a release build has both Apple silicon and Intel joined in one
  executable when Rust has both targets (`rapidr setup` adds them); a debug
  build is this Mac's architecture only.
- Where a build happens: the build cache, outside the output folder —
  `~/Library/Caches/RapidR/build` (macOS), `%LOCALAPPDATA%\RapidR\Cache\build`
  (Windows), `$XDG_CACHE_HOME/rapidr/build` or `~/.cache/rapidr/build`
  (Linux), or `RAPIDR_BUILD_CACHE`. The Rust generated for the program is
  written there and deleted after the build (`--keep-rust` copies it beside
  the app first); cargo's target folder there is shared by every program,
  so RapidR's runtime compiles once (a few minutes) and later builds take
  seconds; each program's own files in it are removed once its app is made.
  Another RapidR version gets its own; one unused for 30 days is removed.
  The whole cache is safe to delete. `CARGO_TARGET_DIR` names another
  target folder.

## The RapidR Runtime

The Runtime runs programs without building them, like a Java runtime: a
`.rrbc` file is a compiled program (`rapidr build-bc`), and source files run
directly. It is the same interpreter inside `--interp` executables.

- **Bytecode format 3** records the oldest runtime a program needs and its
  kind (console or GUI). An older runtime says "this program needs RapidR
  Runtime x.y.z or newer" with a link to the releases; format-2 files still
  run. `rapidr info prog.rrbc` shows it all:

  ```
  apptype: console
  format: 3
  needs-runtime: 2.117.0
  runtime: 2.117.0
  runs: yes
  downloaded: no
  ```

- **File types** (registered per user by the installers, removed by the
  uninstallers):

  | File | Double click | Also |
  |---|---|---|
  | `.rrbc` | runs | |
  | `.rr`, `.bas` | opens in the IDE (with only the Runtime: runs) | Windows: *Run*; macOS / Linux: *Open With > RapidR Runtime* |

  On Windows `.bas` becomes RapidR's by default only if you tick the
  installer's box (other BASICs use `.bas` too); RapidR is always listed
  under *Open with*. Windows uses `rapidrw.exe`, a windowed launcher that
  gives a console program a console and a GUI program none.
- **Downloaded files ask once** before they run: macOS' quarantine,
  Windows' Mark of the Web, and the origin Chrome and Firefox record on
  Linux. The answer is kept per file content in `trusted-files.txt` in
  your RapidR folder (`~/Library/Application Support/RapidR`,
  `%APPDATA%\RapidR`, `~/.config/rapidr`).
- **Console programs opened from the desktop** get a terminal on macOS and
  Linux, which stays open when they end.
- **Scripts**: on macOS and Linux, a `.rr` / `.bas` file whose first line
  is `#!/usr/bin/env rapidr`, made executable, runs as `./script.bas args`.

Standalone `--interp` executables remain for programs shipped to people
who don't have the Runtime.

## `rapidr setup`

```sh
rapidr setup --check      # report only
rapidr setup              # asks before changing anything
rapidr setup --yes        # agrees
```

It reports RapidR's home and the Rust it found, installs the exact Rust
toolchain native builds use (beside yours, through rustup; rustup itself
only if there is no Rust at all, after asking), and offers to link `rapidr`
into `/usr/local/bin` or `~/.local/bin` when it isn't on your PATH
(`--no-path`: don't offer). Uninstalling RapidR leaves Rust alone
(`rustup toolchain uninstall <version>` is yours to run).

## Where RapidR finds its files

`RAPIDR_HOME` if set; else `<the executable's folder>/../lib/rapidr` of an
install (it holds `release.toml`); else the source checkout the command is
run from. `rapidr setup --check` prints which.

## Environment variables

| Variable | |
|---|---|
| `RAPIDR_THEME=name` | the theme for programs that name none ([Components](components.md#themes)) |
| `RAPIDR_RENDERER=cpu\|gpu` | force drawing on the CPU or the GPU (by default the GPU, or the CPU when the only GPU is a software one) |
| `RAPIDR_PRINT_TO=<file or folder>` | printed documents go there as PDFs instead of to a printer |
| `RAPIDR_REGISTRY=<file>` | QREGISTRY keeps its keys in this file instead of the system's (Windows' registry; a per-user store elsewhere) |
| `RAPIDR_CONFIG_DIR=<folder>` | your RapidR folder (the trusted-files list) |
| `RAPIDR_HOME=<folder>` | RapidR's home (above) |
| `RAPIDR_TOOLCHAIN=msvc` | Windows native builds with Microsoft's toolchain |
| `RAPIDR_RUNTIME` | set by the Runtime for the program it runs: the `rapidr` running it |

The `RAPIDR_TEST_*` and `RAPIDR_CAPTURE*` variables drive programs in
RapidR's own tests ([COMPILER_MANUAL.md](../../COMPILER_MANUAL.md)).

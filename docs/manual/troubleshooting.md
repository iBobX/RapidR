# Troubleshooting

## Installing and starting

**macOS: “RapidR Studio” Not Opened — Apple could not verify it is free of
malware** — this release isn't signed. Click **Done**, open **System Settings >
Privacy & Security**, scroll to *Security* and click **Open Anyway** next to
“RapidR Studio” was blocked (then your password or Touch ID). Once is enough.
Right-click > Open doesn't work on macOS 15 and newer. In the Terminal:
`xattr -dr com.apple.quarantine "/Applications/RapidR Studio.app"`. With
screenshots: [Install on macOS](getting-started.md#install-on-macos).

**Windows: "Windows protected your PC"** — SmartScreen, for the same reason.
Click **More info**, then **Run anyway**
([Install on Windows](getting-started.md#install-on-windows)).

**Linux: `N: Download is performed unsandboxed as root as file … couldn't be
accessed by user '_apt'`** — apt's note about a `.deb` in your home folder.
It is harmless: the install went on.

**Linux: `error while loading shared libraries: libssl.so.3`** — RapidR
needs OpenSSL 3: Ubuntu 22.04 or Debian 12 and newer (`sudo apt install
libssl3`). Ubuntu 20.04 and Debian 11 have only OpenSSL 1.1 and aren't
supported.

**Windows: "This app can't run on your PC"** — you have the installer for
the other kind of processor. Look at *Settings > System > About > System
type* and take the `x64` or the `arm64` installer to match
(see [Install on Windows](getting-started.md#install-on-windows)).

**Linux: `apt` says `Unable to locate package rapidr_2.117.0_amd64.deb`** —
put `./` in front of the file name (`sudo apt install
./rapidr_2.117.0_amd64.deb`) so apt reads it as a file in this folder.

**Linux: `dpkg` says "package architecture (amd64) does not match system
(arm64)"** — wrong file for this machine. `dpkg --print-architecture` says
which one to take (`amd64` or `arm64`).

**`rapidr: command not found`** — run `rapidr setup` from the install
(macOS: `/Applications/RapidR Studio.app/Contents/MacOS/rapidr setup`) to link it
into your PATH; on Windows, tick "Add rapidr to PATH" in the installer.

**"This program needs RapidR Runtime x.y.z or newer"** — the program was
compiled by a newer RapidR. Install the newer Runtime (the message links to
the releases).

**A downloaded program asks before it runs** — by design: a program from
the internet asks once; the answer is remembered for that exact file.

## Building

**`Failed to run cargo … Native builds compile with Rust`** — run
`rapidr setup`. Native builds are the only ones that need Rust: `rapidr
run`, `build --interp` and `bundle-bc` don't.

**macOS: `codesign … 3dcube.app: No such process`** — `codesign` reads a
name that starts with a digit as a process number. `rapidr build` now
passes the app's full path and signs it fine; when you run `codesign`
yourself, write `./3dcube.app` (see [The app's
name](building-apps.md#the-apps-name)).

**The first native build takes minutes** — it compiles RapidR's runtime
once; later builds reuse it (keep `CARGO_TARGET_DIR` if you set one).

**`run-time error: 'GetTickCount' is a Windows function (kernel32): this
program calls Windows itself, so it runs on Windows only. For every system,
use TIMER`** — the program calls Windows' DLLs, which only exist on
Windows. Run it there, or use the portable equivalent the message names.

**`'X.DLL' is a 32-bit DLL; RapidR programs are 64-bit, so Windows can't
load it`** — the DLL came with an old program; a 64-bit build of it is
needed (or RapidR's own component for the job).

**`can't find the DLL 'X.DLL'`** — the DLL isn't one of Windows' and isn't
in the current folder or beside the program.

**`the call to F in user32 crashed (access violation …)`** — the DLL was
given something that isn't the pointer or handle it expects: often a TYPE
whose fields hold pointers or handles as `LONG` (64-bit Windows wants them
8 bytes wide), or a control's `Handle` (only a form's is a real window).

**`'SetWindowLongA' is given CODEPTR(WndProc), a callback …`** — SUBs
handed to a DLL to call back aren't supported yet.

**`address … isn't memory of this program`** — PEEK / POKE / MEMCPY reach
only the program's own memory (VARPTR of a variable, an element, a TYPE, a
stream's Pointer) and the console's pages.

**`RUSTSTART ... RUSTEND blocks only work in native builds`** — as it says.

**`--web` fails with "Failed to run wasm-bindgen"** — native web builds need
`rustup target add wasm32-unknown-unknown` and `cargo install
wasm-bindgen-cli --version 0.2.129`. `rapidr bundle-bc` needs neither.

## The program's output

**`PRINT a, b` prints the values joined** — RapidQ's comma works like a
semicolon. Put spaces in yourself, or use `TAB(n)` / `FORMAT$`.

**Numbers print with nine decimals (`3.500000000`)** — RapidQ's format. Use
`FORMAT$("%.2f", x)` or `STRF$` for others.

**A large number prints as `-2147483648`** — PRINT shows a whole number as
a 32-bit integer, as RapidQ does. `STR$(x)` gives `1E10`, `FORMAT$("%.0f",
x)` gives `10000000000`.

**`i = 2.7` stores 2** — stores into integer variables truncate (RapidQ's
rule). Use `ROUND` (which is `INT(x + 0.5)`) when you mean rounding.

**A string with `""` in it comes out wrong** — `""` isn't an escaped quote
in RapidQ-compatible BASIC. Use `CHR$(34)` or `$ESCAPECHARS ON` and `\"`.

**`ON ERROR GOTO` doesn't catch an error** — it's accepted and ignored;
run-time errors end the program with their message and line.

**`[WARN] x.Method() not implemented for type …`** — the component doesn't
have that member on this runtime (for example a web-only component on the
desktop). The program goes on.

## Windows and drawing

**A blank or crashing window in a virtual machine or over remote desktop**
— RapidR draws on the CPU when the only GPU is a software one; if
detection fails, set `RAPIDR_RENDERER=cpu`.

**The program looks different from the classic Windows look** — programs use
the classic look unless they set `$THEME`; check for a `RAPIDR_THEME`
variable in your environment.

## The web

**A blank page from a bundle** — serve the folder over HTTP
(`python3 -m http.server -d folder 8080`); browsers don't load WebAssembly
from `file://`.

**RMySQL / RServerSocket do nothing in the browser** — browsers can't open
raw TCP connections. Call a server of yours through RHttp.

**A file the program reads isn't found on the web** — files beside the
program are built into the page only with the listed extensions (`.csv`,
`.db`, `.txt`, `.png`, …); other names are fetched from the page's server.
See [The web](web.md#files-and-assets).

## Reporting a problem

Open an issue on GitHub (there are forms for bugs, RapidQ compatibility and
test reports): <https://github.com/iBobX/RapidR/issues>. A RapidQ program
that behaves differently under RapidR is a bug — please include it. Report
security problems privately, as [SECURITY.md](../../SECURITY.md) explains.

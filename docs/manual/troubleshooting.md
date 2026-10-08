# Troubleshooting

## Installing and starting

**macOS: "RapidR can't be opened because Apple cannot check it…"** — this
release isn't signed. Right-click the app, choose **Open**, then **Open**
again (or System Settings > Privacy & Security > **Open Anyway**). Once is
enough.

**Windows: "Windows protected your PC"** — SmartScreen, for the same
reason: **More info > Run anyway**.

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
(macOS: `/Applications/RapidR.app/Contents/MacOS/rapidr setup`) to link it
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

**`'GetTickCount' is a Windows API function (LIB "kernel32"); RapidR runs
on every platform and doesn't emulate Windows. Instead, use TIMER`** —
RapidR doesn't run Windows DLL calls. Use the portable equivalent the
message names, or RapidR's components.

**`'x' is an external DLL function (DECLARE ... LIB); the bytecode
interpreter can't call DLLs yet`** — calls into your own libraries work
in native builds (`rapidr build --release`).

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
in RapidQ's BASIC. Use `CHR$(34)` or `$ESCAPECHARS ON` and `\"`.

**`ON ERROR GOTO` doesn't catch an error** — it's accepted and ignored;
run-time errors end the program with their message and line.

**`[WARN] x.Method() not implemented for type …`** — the component doesn't
have that member on this runtime (for example a web-only component on the
desktop). The program goes on.

## Windows and drawing

**A blank or crashing window in a virtual machine or over remote desktop**
— RapidR draws on the CPU when the only GPU is a software one; if
detection fails, set `RAPIDR_RENDERER=cpu`.

**The program looks different from RapidQ's Windows look** — programs use
the classic look unless they set `$THEME`; check for a `RAPIDR_THEME`
variable in your environment.

## The web

**A blank page from a bundle** — serve the folder over HTTP
(`python3 -m http.server -d folder 8080`); browsers don't load WebAssembly
from `file://`.

**QMYSQL / RSERVERSOCKET do nothing in the browser** — browsers can't open
raw TCP connections. Call a server of yours through RHTTP.

**A file the program reads isn't found on the web** — files beside the
program are built into the page only with the listed extensions (`.csv`,
`.db`, `.txt`, `.png`, …); other names are fetched from the page's server.
See [The web](web.md#files-and-assets).

## Reporting a problem

Open an issue on GitHub (there are forms for bugs, RapidQ compatibility and
test reports): <https://github.com/iBobX/RapidR/issues>. A RapidQ program
that behaves differently under RapidR is a bug — please include it. Report
security problems privately, as [SECURITY.md](../../SECURITY.md) explains.

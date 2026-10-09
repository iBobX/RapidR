# Getting started

## Install

Download from [the GitHub releases](https://github.com/iBobX/RapidR/releases).
There are two kinds of package for each system:

- **The RapidR SDK**: the IDE, the `rapidr` command (compiler and runtime),
  everything needed to make standalone executables and web bundles, and
  RapidR's runtime sources for native builds. Install this to write programs.
- **The RapidR Runtime**: only runs RapidR and RapidQ programs (`.rrbc`,
  `.rr`, `.bas`), like a Java runtime. Install this on machines that only
  run programs someone gave you.

The SDK includes the Runtime. Installs are per user and need no
administrator rights (except the Linux `.deb` packages, which install
system-wide).

| System | SDK | Runtime only | Minimum |
|---|---|---|---|
| Windows x64 / ARM64 | `RapidR-2.117.0-windows-<x64\|arm64>-setup.exe` | `RapidR-Runtime-2.117.0-windows-<arch>-setup.exe` | Windows 10 or 11 |
| macOS (one universal app: Apple silicon and Intel) | `RapidR-2.117.0-macos-universal.dmg` | `RapidR-Runtime-2.117.0-macos-universal.dmg` | macOS 11 Big Sur |
| Linux x86_64 / aarch64 | `rapidr-2.117.0-linux-<arch>.tar.gz` or `rapidr_2.117.0_<amd64\|arm64>.deb` | `rapidr-runtime-…` | Ubuntu 22.04, Debian 12 or newer (OpenSSL 3) |
| Web | `rapidr-web-2.117.0.zip`: the web IDE, for any static web host | | a current browser |

- **Windows**: run the installer. It installs into
  `%LOCALAPPDATA%\Programs\RapidR`, adds *RapidR IDE* to the Start menu and,
  if you leave the box ticked, `rapidr` to your PATH. It shows RapidR's
  licence and LEGAL.md first. A box (off by default) makes RapidR the
  default program for `.bas` files.
- **macOS**: open the disk image and drag *RapidR* (and/or *RapidR
  Runtime*) to Applications. The command line is inside the app:
  `/Applications/RapidR.app/Contents/MacOS/rapidr setup` offers to link
  `rapidr` into `/usr/local/bin` or `~/.local/bin`.
- **Linux**: `sudo apt install ./rapidr_2.117.0_amd64.deb`, or unpack the
  `.tar.gz` and run `./install.sh` (into `~/.local`, no root; the folder
  also works as it is: `bin/rapidr`). `~/.local/lib/rapidr/uninstall.sh`
  removes it. Programs need the system's OpenSSL 3 (`libssl3`), ALSA,
  fontconfig and xkbcommon — part of every current desktop distribution.

### Unsigned downloads

This release is not code-signed.

- **macOS**: the first time, right-click the app and choose **Open** (or
  System Settings > Privacy & Security > **Open Anyway**). Once is enough.
- **Windows**: SmartScreen may say "Windows protected your PC": choose
  **More info > Run anyway**.

Each release lists every file's SHA-256 in `SHA256SUMS`
(`shasum -a 256 -c SHA256SUMS`; on Windows `Get-FileHash`).

### Rust, only for native builds

Running programs, the IDE, standalone interpreted executables and web
bundles need nothing else. **Native builds** (your program compiled to
machine code) compile with Rust:

```sh
rapidr setup            # asks first; --yes to agree, --check to only report
```

- With rustup already installed, it adds the exact Rust version RapidR was
  tested with (`1.98.1`) *beside* yours. It never changes your default
  toolchain, never updates or removes one.
- With no Rust at all, it installs rustup (into `~/.cargo` and `~/.rustup`)
  after saying so and asking.
- With a Rust that isn't rustup's, it reports it and changes nothing.
- On Windows the SDK ships its own linker (LLVM-MinGW): no Visual Studio
  needed. On macOS native release builds are universal when both Rust
  targets are installed (`rapidr setup` adds them).
- It also offers to put `rapidr` on your PATH.

Native builds then work offline: the SDK carries the runtime's sources and
every crate they need.

## Your first program

Save this as `hello.bas` (RapidQ's extension; `.rr` works the same):

```basic
PRINT "Hello, World!"
DIM name AS STRING
name = "RapidR"
PRINT "Hi from "; name
```

Run it with the interpreter:

```sh
rapidr run hello.bas
```

```
Hello, World!
Hi from RapidR
```

Or open it in the IDE (`rapidr ide hello.bas`, or double-click it) and press
Run.

## A window

```basic
' A window with a button and a label
$INCLUDE "RAPIDQ.INC"

DECLARE SUB ButtonClick (Sender AS QBUTTON)

CREATE Form AS QFORM
    Caption = "Hello"
    Width = 320
    Height = 160
    Center
    CREATE Label1 AS QLABEL
        Caption = "Not clicked yet"
        Left = 20
        Top = 20
    END CREATE
    CREATE Button1 AS QBUTTON
        Caption = "Click me"
        Left = 20
        Top = 60
        OnClick = ButtonClick
    END CREATE
END CREATE

SUB ButtonClick (Sender AS QBUTTON)
    Label1.Caption = "Clicked!"
END SUB

Form.ShowModal
```

`rapidr run form.bas` opens the window; closing it ends the program. This is
a plain RapidQ program: RapidQ's compiler builds it too. `QFORM` and `RFORM`
are the same component (see [Components](components.md)). `$INCLUDE
"RAPIDQ.INC"` works without the file: RapidR has RapidQ's constants built in.

## Building

| You want | Command | Needs |
|---|---|---|
| Run a program now | `rapidr run prog.bas` | nothing |
| A compiled program for the RapidR Runtime | `rapidr build-bc prog.bas -o prog.rrbc` | nothing |
| A standalone executable (the interpreter inside) | `rapidr build prog.bas --interp` | nothing |
| …for the other Windows architecture, or one macOS slice | `rapidr build prog.bas --interp --target windows-aarch64` | nothing (the SDK ships those runners) |
| A native executable | `rapidr build prog.bas` | Rust (`rapidr setup`) |
| A web bundle (a `.zip` for any static host) | `rapidr bundle-bc prog.bas -o prog-web.zip` | nothing |

Every build writes `THIRD-PARTY-NOTICES.txt` beside the executable (or into
the web bundle). Ship it with your program: it is all the licences of the
components inside ask for ([LEGAL.md](../../LEGAL.md)).

- An interpreted executable is the shipped interpreter with your program
  appended: no Rust needed, the same behaviour as `rapidr run`. On macOS it
  is universal by default.
- A native build translates the program to Rust and compiles it: the
  fastest programs and the smallest executables. `rapidr build prog.bas`
  is the optimized release build, the one you ship; `--debug` compiles
  quicker for debugging. The output folder gets only the app: the
  generated Rust is deleted after the build unless you ask for it
  (`--keep-rust`, or Studio's Project Options).
- Native and interpreted builds behave the same: the conformance suite runs
  every case both ways, and on the web.

Details: [The CLI and the RapidR Runtime](cli-and-runtime.md).

## The web

```sh
rapidr bundle-bc form.bas -o form-web.zip
unzip form-web.zip -d form-web
python3 -m http.server -d form-web 8080      # any static web server
```

Open `http://localhost:8080`: the same window, drawn by the same UI kernel
in a canvas. See [The web](web.md).

## The IDEs

- **The desktop IDE** (SDK: Start menu > RapidR IDE, the RapidR app on
  macOS, *RapidR IDE* on Linux, or `rapidr ide [file]`) is written in RapidR
  itself: a form designer, a code editor with BASIC colours, Run. It is an
  early version; the full IDE (RapidR Studio: IntelliSense, a debugger,
  linked data components, AI assistance) is being built
  ([docs/ide-plan.md](../ide-plan.md)). Its debugger works today:
  [Debugging in RapidR Studio](debugging.md).
- **The web IDE** (`rapidr-web-2.117.0.zip`, any static host): RapidR
  Studio in the browser — the designer, the code editor and Run — with no
  server-side code (apps and web bundles are built on the desktop for now).

## Where next

- [The language](language.md), with the RapidQ rules that surprise people
  coming from other BASICs (PRINT's comma, numbers, integer stores).
- [The examples](../../examples/README.md).
- RapidQ's own documentation, for its components' members.

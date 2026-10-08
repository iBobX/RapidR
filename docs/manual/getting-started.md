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

- **Windows**: see [Install on Windows](#install-on-windows) below.
- **macOS**: open the disk image and drag *RapidR* (and/or *RapidR
  Runtime*) to Applications. The command line is inside the app:
  `/Applications/RapidR.app/Contents/MacOS/rapidr setup` offers to link
  `rapidr` into `/usr/local/bin` or `~/.local/bin`.
- **Linux**: see [Install on Ubuntu or Debian](#install-on-ubuntu-or-debian-the-deb-package)
  below for the `.deb`. The `.tar.gz` installs for your user only, with no
  root: unpack it and run `./install.sh` (into `~/.local`; the folder also
  works as it is: `bin/rapidr`). `~/.local/lib/rapidr/uninstall.sh` removes
  it. Programs need the system's OpenSSL 3 (`libssl3`), ALSA, fontconfig
  and xkbcommon, which every current desktop distribution has.

### Install on Windows

1. **Pick the right file.** Open *Settings > System > About* and look at
   *System type*: "64-bit operating system, x64-based processor" needs
   `RapidR-2.117.0-windows-x64-setup.exe`; "ARM-based processor" (for
   example Windows 11 in a Mac's virtual machine, or a Snapdragon PC) needs
   `RapidR-2.117.0-windows-arm64-setup.exe`. To only run programs, take the
   `RapidR-Runtime-…` file of the same kind instead.
2. **Run the installer** (double-click it). You are not asked for an
   administrator password: RapidR installs for your user, into
   `%LOCALAPPDATA%\Programs\RapidR`. The first time, Windows may say
   "Windows protected your PC" because this release isn't code-signed: choose
   **More info**, then **Run anyway** (see [Unsigned downloads](#unsigned-downloads)).
3. **Read and accept** RapidR's licence, read the legal notes (LEGAL.md),
   and click **Next** (a page for the install folder shows up the first
   time; the default is fine).
4. **Choose the options** on the "Select Additional Tasks" page:
   - *Add rapidr to PATH (for the command line)* is ticked. Leave it so
     `rapidr` works in any Command Prompt or PowerShell window you open
     afterwards (windows that were already open don't see it).
   - *Open .bas files with RapidR by default* is off. Tick it if RapidR
     should open your RapidQ `.bas` files when you double-click them. Files
     ending in `.rrbc` and `.rr` always belong to RapidR.
5. **Click Install**, then **Finish**.

The SDK adds **RapidR IDE** to the Start menu: open the menu, type "RapidR",
and press Enter to start the IDE. To check the command line, open a new
Command Prompt and type:

```
rapidr version
```

```
RapidR 2.117.0
```

Then try the first program below (`rapidr run hello.bas`).

To remove RapidR: *Settings > Apps > Installed apps*, find "RapidR 2.117.0",
and choose **Uninstall**. It takes away the files, the Start menu entry, the
file types and the PATH entry. The programs you wrote are yours and stay.

### Install on Ubuntu or Debian (the .deb package)

The `.deb` is the install for Ubuntu 22.04 or newer and Debian 12 or newer.
It is system-wide, so it asks for your password (`sudo`).

1. **Pick the right file.** In a terminal, run `dpkg --print-architecture`.
   It prints `amd64` (most PCs; take `rapidr_2.117.0_amd64.deb`) or `arm64`
   (Raspberry Pi 4 and 5, ARM servers, Ubuntu in a Mac's virtual machine;
   take `rapidr_2.117.0_arm64.deb`). To only run programs, take
   `rapidr-runtime_2.117.0_<arch>.deb` instead.
2. **Install it with apt**, from the folder you downloaded it to. Keep the
   `./` in front of the name: it tells apt that this is a file, and apt then
   fetches whatever RapidR needs (OpenSSL 3, fontconfig and a few more) by
   itself:

   ```sh
   cd ~/Downloads
   sudo apt install ./rapidr_2.117.0_arm64.deb
   ```

3. **Check it:**

   ```sh
   rapidr version
   ```

   ```
   RapidR 2.117.0
   ```

4. **Start the IDE** from the applications menu (*RapidR IDE*, under
   Development), or from a terminal with `rapidr ide`. `rapidr run hello.bas`
   runs a program (the first program below). In the file manager,
   double-clicking a compiled `.rrbc` program runs it; a `.rr` or `.bas`
   source file offers *RapidR IDE* under *Open With*.

What the package puts where: the `rapidr` command in `/usr/bin`; RapidR's
other files (the runtime's sources for native builds, the IDE, a few example
programs) in `/usr/lib/rapidr`; the licence, the notices and this manual
(`manual/`) in `/usr/share/doc/rapidr`; and the menu entry, file types and
icons in the system's usual places.

The SDK package (`rapidr`) and the Runtime package (`rapidr-runtime`) are
alternatives: installing one removes the other, because both provide the
`rapidr` command. To remove RapidR entirely:

```sh
sudo apt remove rapidr
```

If you would rather not use `sudo`, the `.tar.gz` installs under your own
folder (`./install.sh`, above).

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
| A native executable | `rapidr build prog.bas --release` | Rust (`rapidr setup`) |
| A web bundle (a `.zip` for any static host) | `rapidr bundle-bc prog.bas -o prog-web.zip` | nothing |

Every build writes `THIRD-PARTY-NOTICES.txt` beside the executable (or into
the web bundle). Ship it with your program: it is all the licences of the
components inside ask for ([LEGAL.md](../../LEGAL.md)).

- An interpreted executable is the shipped interpreter with your program
  appended: no Rust needed, the same behaviour as `rapidr run`. On macOS it
  is universal by default.
- A native build translates the program to Rust and compiles it: the
  fastest programs and the smallest executables. `rapidr build prog.bas`
  alone is a quick debug build; `--release` is the optimized one. The
  generated Rust project is kept in `prog_rust/` beside the source.
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
  ([docs/ide-plan.md](../ide-plan.md)).
- **The web IDE** (`rapidr-web-2.117.0.zip`, any static host): RapidR
  Studio in the browser — the designer, the code editor and Run — with no
  server-side code (apps and web bundles are built on the desktop for now).

## Where next

- [The language](language.md), with the RapidQ rules that surprise people
  coming from other BASICs (PRINT's comma, numbers, integer stores).
- [The examples](../../examples/README.md).
- RapidQ's own documentation, for its components' members.

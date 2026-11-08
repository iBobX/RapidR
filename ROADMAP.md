# RapidR Roadmap

**RapidR is a modern BASIC with its own IDE, RapidR Studio.** One program runs
as a native app on Windows, macOS and Linux, as a standalone app with nothing
to install, or as a web app in any browser. Our aim for 1.0: the fastest, most
secure and easiest BASIC there is, accessible to everyone, with AI that helps
you build and helps you learn, on any computer, including old and cheap ones.

This page says where RapidR is going, in order. There are no dates: each
release ships when it's ready and has been checked on every system.
What changed in each version is in [CHANGELOG.md](CHANGELOG.md).

## Where we are: Preview 6.1 (2.123.1)

- [x] **RapidR Studio** on the desktop and in the browser: form designer, code
  editor with completion and hover help, a debugger whose values open like
  trees, Help (F1), and data shown live in the designer.
- [x] **Three runtimes, one behaviour**: native code, the RapidR Runtime
  (bytecode, and standalone apps) and the web, checked against each other.
- [x] **RapidR's own UI kernel**: the same look on every system, sharp on
  high-DPI screens, light, dark and high contrast, accessible to screen readers
  and the keyboard.
- [x] **Components** for windows, dialogs, grids, graphics, sound, MIDI, 2D and
  3D, databases (SQLite, MySQL), networking, serial ports, archives, and data
  science (`RNum`, `RDataFrame`, `RPlot`).
- [x] **RAI**, the assistant: describe an app and it builds it, checks it and
  fixes its own errors; **Coach mode** gives hints, not answers; **School
  Lock** for teachers; free AI models on your own computer, found and tried
  for you.
- [x] **Versions**: save a version, see what changed, go back; local history;
  crash-safe unsaved work; RapidR's own crash reports, kept on your computer.
- [x] **Studio, the compiler and the manual in English and Spanish**; Studio
  and the compiler's messages also in Brazilian Portuguese and Simplified
  Chinese (beta: AI translations, corrections welcome).
- [x] **Big numbers**: `INT64` / `UINT64` keep every digit wherever they go,
  with `UINT32` and the bit functions (`LOWORD`, `HIWORD`, `GETBITS`...).
- [x] **Old PCs (alpha)**: Windows 7, 8 and 8.1, 32-bit; and plain Windows XP
  SP3 as an unofficial test.

New in Preview 6 (2.123.0) and Preview 6.1 (2.123.1):

- [x] **Fast**: hot code is compiled to machine code while a program runs, in
  Studio's Run too (64-bit Windows, macOS and Linux): 4 to 20 times faster and
  close to a native build. The interpreter runs typed code 2.5 to 6 times
  faster everywhere, and Studio never waits for the code check.
- [x] **Right-click menus everywhere in Studio**, by mouse, keyboard and
  screen reader; **`RNotify`** and **`RNotifyBar`** for small messages that
  come and go and a one-line question with its buttons.
- [x] **AI agents use RapidR the right way**: Studio's MCP server tells any
  agent, even a small one, how RapidR is used; RAI can run your program, read
  its output and debug, and asks first.
- [x] **Build for every system, from the computer you are on**: Windows (x64,
  ARM64, 32-bit), macOS, Linux (x64, ARM64) and the web, from the desktop
  Studio's Run > Build or `rapidr build`. A Mac app is signed by RapidR itself:
  no Mac needed.
- [x] **One standard for the Windows API**: `$INCLUDE "RWin32.inc"`, built in,
  generated from the Windows metadata Microsoft publishes.
- [x] **New components and small things**: `RMIDIPort` (MIDI in and out, with
  SysEx), `RFormEx`, `RAgentTools` (your web app offers its own tools to AI
  agents in the browser), `BringToFront`, `CopyMode`, and data grids that read
  as tables to screen readers.
- [x] **Safer**: four security audits found 13 problems, all fixed; runs RAI
  starts can't use DLL calls or `SHELL` unless you allow it.

## Next: Preview 7

The rest of the speed release, and building for every system from the browser.

- [ ] **Export anywhere, from the web Studio too**: the desktop Studio builds
  for every system since 2.123.1; Studio in the browser builds the web app
  today, and the Windows, macOS and Linux apps come next.
- [ ] **Portuguese and Chinese: the manual and Help**: Studio and the messages
  are there (beta); the manual and Help still show in English.
- [ ] **Fast arrays and maths**: typed arrays are plain memory in native builds;
  next, the interpreter and the web, and quick `SIN`, `COS` and friends in
  every runtime.
- [ ] **A real-time audio engine** on its own thread: synthesizers, effects and
  music with no glitches; **`RAudioIn`** for the microphone and line-in.
- [ ] **Bigger numbers**: `INT64` / `UINT64` are checked everywhere; next,
  `INT128` / `UINT128` and whole numbers of any size.
- [ ] **The best error messages**: mistakes that used to pass in silence are
  now errors that name their line and the fix; next, "did you mean...?", a
  suggested fix on every message, warnings said once, clear for a student, an
  engineer and an AI agent alike.
- [ ] **RAI connects to other apps' tools**: RAI and your programs can use
  MCP servers (design tools, 3D apps, databases...), with what each one may
  do chosen by you, and every call shown.
- [ ] **RAI calls your app's own tools**: your program gives RAI its own
  actions, checked before they run.
- [ ] **Speed is a release gate**: the interpreter and the user interface are
  held to speed budgets in the tests; next, every component gets a speed
  budget and a benchmark, measured on old and cheap machines too.

## Preview 8: Connected

- [ ] **`RSpeech`, `RTutor` and `RCaptions`**: programs that speak, guided
  tours with a virtual cursor, and captions.
- [ ] **`RWebServer`**, security first: control a program from a phone's
  browser.
- [ ] **Boards over USB, Bluetooth and Wi-Fi** (ESP32, micro:bit, M5Stack), and
  dashboard components: knobs, switches, gauges, displays.
- [ ] **A lab kit for schools** that works with no board at all: oscilloscope,
  signal generator, meters, LEDs, 7-segment displays, a logic analyser.
- [ ] **More RAI**: asks permission in plain words, tests risky code safely (a
  first sandbox for the runs RAI starts is in), Pair and Quiz modes next to
  Coach, and a chat component for your apps.
- [ ] **More of Versions**: experiments, and publishing to GitHub.

## Preview 9: On the board

- [ ] **RapidR running on the board itself** (ESP32, micro:bit, M5Stack):
  flashed from Studio, debugged on the board, live pins.

## Preview 10: Science

- [ ] A complete data-science stack of our own at full speed: data frames,
  arrays, plots, statistics, machine learning.
- [ ] More data sources as linked components: spreadsheets, Parquet,
  PostgreSQL.
- [ ] **`RInterpreter`**: RapidR as a safe, embeddable script engine.

## Preview 11 and on to 1.0

- [ ] Layout made easy: a stack panel, a grid panel, properties that follow
  variables.
- [ ] **A native look** (`$THEME native`): programs that look like the system
  they run on.
- [ ] Networking clients (SSH, SFTP), then servers.
- [ ] **Lean builds**: each program carries only the components it uses.
- [ ] **Signed installers** for Windows and macOS.
- [ ] **RapidR 1.0**: security, accessibility and hands-on audits on every
  system, and the complete manual.

## After 1.0

- [ ] **Truly native controls** (`$UI native`) on macOS, Windows, iOS and
  Android, and **mobile apps**.
- [ ] Calls into native libraries on macOS and Linux, as DLL calls work on
  Windows today.
- [ ] Video courses in four languages.

## Compatibility

RapidR includes a **compatibility layer**: programs written for RapidQ, and
much classic VB-style code, run as they are. `rapidr import-rapidq` converts a
program to RapidR's own names, with a report of every change. A program that
brings its own `WIN32API.INC` runs as it is; `RWin32.inc` is RapidR's built-in
standard for the Windows API, there when you want it. Details:
[compatibility](docs/manual/compatibility.md), [importing older programs](docs/manual/importing-rapidq.md)
and [the Windows API](docs/manual/windows-api.md).

## How we work

- **Never silently wrong**: every program compiles correctly or gets a clear
  message with its line.
- **One behaviour** on native, the RapidR Runtime and the web.
- **Secure by default**: programs, imported projects and AI output never get
  more than they asked for. Your keys stay in your system's keychain.
- **Accessible by default**: every program works with the keyboard and screen
  readers without extra work.
- **Fast, easy, beautiful, legal**: everything RapidR ships is open source
  under licences that let you sell your apps.
- **Seen working**: every feature is run by hand on every system before it
  ships.

Ideas and requests are welcome in
[Discussions](https://github.com/iBobX/RapidR/discussions); bugs in
[Issues](https://github.com/iBobX/RapidR/issues).

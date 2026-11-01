# RapidR Roadmap

**RapidR is a modern BASIC with its own IDE, RapidR Studio.** One program runs
as a native app on Windows, macOS and Linux, as a standalone app with nothing
to install, or as a web app in any browser. Our aim for 1.0: the fastest, most
secure and easiest BASIC there is, accessible to everyone, with AI that helps
you build and helps you learn, on any computer, including old and cheap ones.

This page says where RapidR is going, in order. There are no dates: each
release ships when it's ready and has been checked on every system.
What changed in each version is in [CHANGELOG.md](CHANGELOG.md).

## Where we are: Preview 5 (2.122.1)

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
- [x] **Studio, the compiler and the manual in English and Spanish.**
- [x] `INT64` / `UINT64`.
- [x] **Old PCs (alpha)**: Windows 7, 8 and 8.1, 32-bit.

## Next: the speed release

Before any new feature, everything RapidR already does gets fast.

- [ ] **Run in Studio at native-class speed**: a faster interpreter now, then
  a compiler that turns your hottest code into machine code while it runs.
- [ ] **Fast arrays and maths**: typed arrays as plain memory, quick `SIN`,
  `COS` and friends, in every runtime.
- [ ] **A real-time audio engine** on its own thread: synthesizers, effects and
  music with no glitches; **`RAudioIn`** for the microphone and line-in.
- [ ] **Big numbers**: a full check of `INT64` / `UINT64` everywhere, then
  `INT128` / `UINT128` and whole numbers of any size.
- [ ] **The best error messages**: "did you mean…?", suggested fixes, warnings
  said once, clear for a student, an engineer and an AI agent alike.
- [ ] **AI agents use RapidR the right way**: Studio's MCP server guides any
  agent, even a small one, through RapidR's help, examples and checks.
- [ ] **Speed is a release gate**: every component has a speed budget and a
  benchmark, measured on old and cheap machines too.

## Preview 6: Connected

- [ ] **`RSpeech`, `RTutor` and `RCaptions`**: programs that speak, guided
  tours with a virtual cursor, and captions.
- [ ] **`RWebServer`**, security first: control a program from a phone's
  browser.
- [ ] **Boards over USB, Bluetooth and Wi-Fi** (ESP32, micro:bit, M5Stack), and
  dashboard components: knobs, switches, gauges, displays.
- [ ] **A lab kit for schools** that works with no board at all: oscilloscope,
  signal generator, meters, LEDs, 7-segment displays, a logic analyser.
- [ ] **More RAI**: asks permission in plain words, tests risky code safely,
  Pair and Quiz modes next to Coach, and a chat component for your apps.
- [ ] **More of Versions**: experiments, and publishing to GitHub.
- [ ] **Portuguese and Chinese**: Studio, the messages and the manual.

## Preview 7: On the board

- [ ] **RapidR running on the board itself** (ESP32, micro:bit, M5Stack):
  flashed from Studio, debugged on the board, live pins.

## Preview 8: Science

- [ ] A complete data-science stack of our own at full speed: data frames,
  arrays, plots, statistics, machine learning.
- [ ] More data sources as linked components: spreadsheets, Parquet,
  PostgreSQL.
- [ ] **`RInterpreter`**: RapidR as a safe, embeddable script engine.

## Preview 9 and on to 1.0

- [ ] Layout made easy: a stack panel, a grid panel, properties that follow
  variables.
- [ ] **A native look** (`$THEME native`): programs that look like the system
  they run on.
- [ ] Networking clients (SSH, SFTP), then servers.
- [ ] **Lean builds**: each program carries only the components it uses.
- [ ] **Export anywhere**: any Studio, even the web one, builds for Windows,
  macOS, Linux and the web.
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
program to RapidR's own names, with a report of every change. Details:
[compatibility](docs/manual/compatibility.md) and [importing older programs](docs/manual/importing-rapidq.md).

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

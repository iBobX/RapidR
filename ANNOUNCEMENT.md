# RapidR 2.117.0 — Announcement Drafts

Three versions of the same announcement, for different venues. The facts are
the release notes' ([docs/release-notes/v2.117.0.md](docs/release-notes/v2.117.0.md));
keep the two in step. Wording: RapidR is *compatible with* RapidQ — an
original implementation, written from the ground up in pure Rust.

---

## A. RapidQ user groups and forums (warm, for RapidQ users)

> **RapidR 2.117.0 — your RapidQ programs, on today's computers**
>
> Long-time RapidQ users, this one is for you. RapidR runs and builds
> RapidQ programs. It is compatible with RapidQ: an original implementation,
> written from the ground up in pure Rust, and an existing RapidQ program
> behaves as it does under RapidQ. This is its first public release.
>
> Your program runs three ways from the same source:
>
> - **natively**, as an executable of its own;
> - **interpreted**, by the RapidR Runtime (or as a standalone executable
>   that carries it) — no Rust or anything else to install;
> - **in a browser**, the same program on a web page.
>
> What that means for your code:
>
> - **Checked against RapidQ itself.** RapidQ's own compiler (RC.EXE) runs in
>   a Windows VM as the ground truth: numbers, PRINT, STR$, rounding, colours
>   and more print exactly what RapidQ prints, on all three runtimes.
> - **The whole language and every RapidQ object but OLE**: TYPEs with
>   methods and inheritance, GOTO / GOSUB, DATA / READ, `$RESOURCE`, the
>   visual components, dialogs, menus, QREGISTRY, the printer, sockets,
>   QMYSQL, the tray icon, QCGI, QCOMPORT, QMIDI, QWAVE, QVIDEO, the include
>   libraries' components (QFORMMDI, QBEVEL, QDIGDISPLAY, …) built in — and
>   DirectX, Direct3D retained mode included, everywhere, the browser too.
> - **RapidQ's look on macOS, Windows and Linux**, drawn by RapidR's own UI
>   kernel: sharp on high-DPI screens, readable by screen readers, with
>   themes (`$THEME Dark`, `Modern`, …) when you want them.
> - **New things, without changing what RapidQ programs do**: SQLite with
>   parameter binding, JSON and HTTP, data frames, arrays and charts
>   (`RDataFrame`, `RNum`, `RPlot`), responsive layouts.
>
> Installers for Windows, macOS and Linux, plus the web IDE as a static
> `.zip`. Install, open the IDE, write a program, press Run.
>
> Please try your old programs and tell me what differs:
> https://github.com/iBobX/RapidR/issues — that is how RapidR gets to
> "behaves as RapidQ" for everyone's code. RapidR is MIT-licensed and
> built by one developer; if it's useful to you, you can
> [buy me a coffee](https://www.buymeacoffee.com/roanbema).

---

## B. Hacker News / Reddit (one paragraph, technical)

> **Show HN: RapidR — RapidQ-compatible BASIC on a native compiler, a VM
> and the web, in pure Rust**
>
> RapidR runs and builds programs written for RapidQ, the early-2000s BASIC
> for GUI programs. It is an original implementation, written from the
> ground up in pure Rust and checked against RapidQ's own compiler running
> in a Windows VM. One source has three runtimes: a native compiler (BASIC
> → Rust → an executable), a bytecode VM (the RapidR Runtime, also as
> standalone executables, no toolchain needed) and the browser (the VM and
> runtime as WebAssembly). Every window is drawn by RapidR's own UI kernel
> (winit, vello, parley, AccessKit) — RapidQ's classic Windows look on
> macOS, Windows, Linux and a canvas in the browser, compared pixel by
> pixel and accessibility tree by tree across them — and DirectX / Direct3D
> programs run on its own software rasterizer. Extensions: SQLite
> everywhere (compiled to wasm on the web) with parameter binding, JSON,
> HTTP, data frames and charts. Every build writes the
> `THIRD-PARTY-NOTICES.txt` its dependency graph needs; nothing copyleft
> or cryptographic is compiled into a program. MIT.

---

## C. GitHub release / README banner

> # RapidR 2.117.0
>
> **RapidR's first public release.** RapidR runs and builds RapidQ
> programs, compatible with RapidQ on all three of its runtimes — the
> **native compiler**, the **interpreter** (the RapidR Runtime) and the
> **web** — as an original implementation written from the ground up in
> pure Rust.
>
> **Highlights**
>
> - **RapidQ compatibility, checked against RapidQ's own compiler**: the
>   whole language, every RapidQ object but OLE, DirectX and Direct3D
>   retained mode; the 123 programs of RapidQ's example corpus that use the
>   API its Windows and Linux versions shared all compile and run alike
>   natively and interpreted.
> - **RapidR's own UI kernel** on the desktop and the web: RapidQ's look
>   everywhere, themes, high-DPI, screen-reader accessibility, responsive
>   layouts.
> - **The RapidR Runtime and installers** for Windows (x64, ARM64), macOS
>   (universal) and Linux (x86_64, aarch64), and the web IDE as a static
>   `.zip`. Programs run and standalone executables build with no Rust
>   installed; `rapidr setup` installs Rust once for native builds.
> - **Extensions**: `RSQLite` and QMYSQL with parameter binding, `RNum`,
>   `RDataFrame`, `RPlot`, `RJson`, `RHttp`, and web-only components.
> - **Your programs are yours**: every build writes its
>   `THIRD-PARTY-NOTICES.txt`; RapidR is MIT, everything it ships is
>   permissively licensed. RapidR is compatible with RapidQ and not
>   affiliated with its author ([`LEGAL.md`](LEGAL.md)).
>
> **Known limits**: OLE / COM objects compile but do nothing; Windows API
> calls (`DECLARE … LIB "user32"`) aren't emulated; the installers aren't
> code-signed; the IDE is early — IntelliSense, a debugger and AI
> assistance are the next milestone ([ROADMAP.md](ROADMAP.md)).
>
> Downloads, checksums and the full list:
> [the release notes](docs/release-notes/v2.117.0.md) ·
> [the user manual](docs/manual/README.md) ·
> [CHANGELOG.md](CHANGELOG.md)

# The RapidR user manual

RapidR runs and builds RapidQ BASIC programs, and goes further. It is an
original implementation written from the ground up in pure Rust, compatible
with RapidQ: an existing RapidQ program behaves as it does under RapidQ, on
three runtimes — native executables, the interpreter (the RapidR Runtime)
and the web browser.

This manual describes RapidR **2.117.0**. It states what the code does today;
where something isn't done yet, it says so.

| Chapter | What's in it |
|---|---|
| [Getting started](getting-started.md) | Install, your first console and GUI programs, running, building executables, the web |
| [The language](language.md) | Program structure, types, variables, arrays, operators, control flow, SUBs and FUNCTIONs, TYPEs and objects, the preprocessor, files, the console — and where RapidQ-compatible BASIC differs from other BASICs |
| [Components and objects](components.md) | `CREATE`, properties, methods and events; RapidR's names and RapidQ's names; the global objects; themes, high-DPI and accessibility |
| [The CLI and the RapidR Runtime](cli-and-runtime.md) | Every `rapidr` command, the kinds of builds, the Runtime, file types, `rapidr setup`, environment variables, the notices builds carry |
| [Building apps and their icons](building-apps.md) | `Name.app`, the `.exe`'s icon and version, `Name.AppDir`; your icon or RapidR's; Studio's Build |
| [The web](web.md) | Running programs in a browser: bundles, `--web` builds, files and assets, web-only components |
| [Databases](databases.md) | RSQLite and RMySQL, parameter binding, events |
| [Data science](data-science.md) | RNum, RDataFrame, RPlot, RJson |
| [DirectX and media](directx-and-media.md) | RDXScreen and the Direct3D objects, sound, joysticks, RMIDI / RWave / RVideo / RCDAudio, the system tray, serial ports, CGI, downloads |
| [Differences from RapidQ, and extensions](differences.md) | What RapidR does that RapidQ doesn't, what it does differently on purpose, and what it doesn't do |
| [Troubleshooting](troubleshooting.md) | Common messages and what to do about them |

Reference tables, generated from RapidR's language registry (`rapidr lang
export --manual`):

- [Components: RapidR's names and RapidQ's names](reference/components.md)
- [Every component's properties, methods and events](reference/members.md)
- [Built-in functions](reference/builtins.md)
- [Statements, directives, keywords and types](reference/statements.md)
- [Constants](reference/constants.md)
- [Data-science members](reference/data-science.md)

Elsewhere:

- [The examples](../../examples/README.md)
- [CHANGELOG.md](../../CHANGELOG.md): what changed in each version
- [LEGAL.md](../../LEGAL.md): what you may do with RapidR and the programs you build
- The reference for every component and its members is
  [reference/](reference/components.md), in RapidR's own words. RapidQ's own
  documentation remains useful for RapidQ's history and for older programs.

The RapidR SDK installs this manual with its other documents: on Windows in
`share\doc\rapidr\manual` under the install folder
(`%LOCALAPPDATA%\Programs\RapidR`), on macOS inside the app
(`RapidR.app/Contents/Resources/doc/manual`), on Linux in
`/usr/share/doc/rapidr/manual` (the `.deb`) or
`~/.local/share/doc/rapidr/manual` (the `.tar.gz`'s `install.sh`).

# The RapidR user manual

RapidR is a modern BASIC with its own IDE, RapidR Studio: an original
implementation written from the ground up in pure Rust, on three runtimes —
native executables, the interpreter (the RapidR Runtime) and the web browser.
Programs written for RapidQ run too.

This manual describes RapidR **2.119.0** (Preview 2). It states what the code
does today; where something isn't done yet, it says so.

**New here?** Read [Getting started](getting-started.md) (its first three
lines say which download to take and how to run and build), then
[Designing forms in RapidR Studio](designing-forms.md). Wondering what
RapidR is good at? [Why RapidR](why-rapidr.md).

## Start

| Chapter | What's in it |
|---|---|
| [Why RapidR](why-rapidr.md) | What RapidR does best, with only measured numbers (their conditions and the scripts that made them), and what it doesn't claim |
| [Showcase](showcase.md) | Bigger programs written from an idea alone, each with a picture and its best lines explained: fireworks, a MIDI visualizer that reads the song file itself, and more |
| [Getting started](getting-started.md) | Which download to take (the SDK or the Runtime), installing on Windows, macOS and Linux, your first console and GUI programs, running (`rapidr run`), compiling (`build-bc`), building executables (`build`) and the web |

## RapidR Studio

| Chapter | What's in it |
|---|---|
| [The code editor](studio-editor.md) | Completion, Ctrl+Space, parameter info, hover, F12 Go to Definition, problems and quick fixes, the keyboard shortcuts |
| [Projects](studio-projects.md) | The Project pane: forms come from your code (several in one file), File > Open Recent, reopening the last project, the PATH your programs get (macOS, Linux) |
| [Designing forms](designing-forms.md) | A tutorial (a program of two forms, step by step), the toolbox, moving and aligning, the inspector and events, the menu and Tab-order editors, programs of several forms, the code the designer writes |
| [Debugging](debugging.md) | Running and stopping, breakpoints (conditions, hit counts, log messages), stepping, Step Into Specific, Set Next Statement, Pause, values as trees (data tips, Variables, Watch), the Call Stack, Immediate, run-time errors, how fast it runs, the desktop and the browser |
| [The AI assistant and RAI](ai-assistant.md) | Setting up a provider and a key, asking, changes as diffs you accept, "Build me…", what was sent, privacy and tokens; the `RAI` component for your own programs, and keeping keys out of them |
| [Markdown files and RMarkdownView](markdown.md) | `.md` files formatted in RapidR Studio (Preview \| Source), and Markdown shown in your own programs, its code blocks coloured by language (SQL, BASIC, JSON …) |
| [Versions: never lose your work](versions.md) | Saving versions, the timeline, going back, experiments, shared folders, safety points and Undo, the secret check — in plain words with Git's names beside them (the Studio panel comes in Preview 4); `RDiffView`'s text, form and table views in your own programs |

## The language and its components

| Chapter | What's in it |
|---|---|
| [The language](language.md) | Program structure, types, variables, arrays, operators, control flow, SUBs and FUNCTIONs, TYPEs and objects, the preprocessor, files, the console — and where RapidR's BASIC differs from other BASICs |
| [Your program in every language](translations.md) | `TR$` and `TRN$`, `Application.Language`, translation files (`lang/es.po`) beside the program, plurals, placeholders, falling back from `es-AR` to `es` |
| [Components and objects](components.md) | `CREATE`, properties, methods and events; RapidR's names and the older Q names; the global objects; themes, high-DPI and accessibility |
| [Databases](databases.md) | RSQLite and RMySQL, parameter binding, events, encrypted MySQL connections (`SSLMode`) |
| [Keeping secrets](secrets.md) | `RSecrets`: passwords and keys in the OS keychain, each program its own, never shown by the debugger or to an AI; none in a browser |
| [Your own SVG icons](svg-icons.md) | `RImageList.AddSVGFile`, SVGs that take the theme's colours (`currentColor`, RapidR's colour tokens), SVGs built in with `$RESOURCE` |
| [Databases](databases.md) | RSQLite and RMySQL, parameter binding, events; background queries (`QueryAsync`, `Cancel`) streaming a million rows into an `RDBGrid`; column names and types; the SQL editor |
| [The code editor in your programs](code-editor.md) | `RCodeEditor`: languages, files, completion; its SQL mode (dialects, completion from your tables, Ctrl+Enter runs the statement) |
| [DirectX and media](directx-and-media.md) | RDXScreen and the Direct3D objects, sound, joysticks, RMIDI / RWave / RVideo / RCDAudio, the system tray, CGI, downloads |
| [Networking: sockets and a local web server](network.md) | Servers on `127.0.0.1` (this computer only) or the network; `RServerSocket` by lines, bytes or whole HTTP requests; `RHttpServer`: a local API, Server-Sent Events, the token, the requests it refuses by itself, an MCP server |
| [Recipes for systems programs](recipes-systems.md) | Ten small programs for a machine's insides, one technique each, one tiny computer between them: memory as a byte array, bits and flags, a CPU that fetches and decodes, the stack, a hex dump, a disassembler, a binary file, video memory on a canvas, a keyboard matrix, cycles per frame; how fast RapidR runs them |
| [Serial ports and IoT boards](serial-ports.md) | RComPort (older name QCOMPORT): opening ports, listing them with their USB IDs, lines in, plug events, resetting an ESP32 through DTR / RTS, Web Serial |

## Data science and rich text

| Chapter | What's in it |
|---|---|
| [Data science with RapidR](data-science-with-rapidr.md) | A step-by-step start: a table from a file on a form, live in the designer without code, the current row, filtering and sorting, a million rows, JSON records |
| [Data science](data-science.md) | RNum, RDataFrame, RPlot, RJson and the linked data components (RCSV, RDataSource, RDBGrid): every method, how fast, the CSV Explorer tutorial |
| [Rich text: RRichEdit](rich-text.md) | Formatting text from a program (`SelAttributes`), what the user types, paragraphs, RTF files, screen readers |

## Building, importing and shipping

| Chapter | What's in it |
|---|---|
| [Building apps and their icons](building-apps.md) | `Name.app`, the `.exe`'s icon and version, `Name.AppDir`; your icon or RapidR's; Studio's Build; Build Web App |
| [Importing older programs](importing-rapidq.md) | A copy of an older program (`.bas`, `.rqw`, `.rqb`, `.rq`) with RapidR's names, checked to compile to the same bytecode: in RapidR Studio and with `rapidr import-rapidq`; include files |
| [The web](web.md) | Running programs in a browser: bundles, `--web` builds, files and assets, web-only components, windows bigger than the page |
| [The CLI and the RapidR Runtime](cli-and-runtime.md) | Every `rapidr` command, the kinds of builds, the Runtime, file types, `rapidr setup`, environment variables, the notices builds carry |

## When something differs, or goes wrong

| Chapter | What's in it |
|---|---|
| [Differences and extensions](differences.md) | What RapidR adds, what it does differently on purpose, and what it doesn't do |
| [Troubleshooting](troubleshooting.md) | Common messages and what to do about them |

## Reference tables

Generated from RapidR's language registry (`rapidr lang export --manual`):

- [Components](reference/components.md), and [their older Q names](reference/classic-names.md)
- [Every component's properties, methods and events](reference/members.md)
- [Built-in functions](reference/builtins.md)
- [Statements, directives, keywords and types](reference/statements.md)
- [Constants](reference/constants.md)
- [Data-science members](reference/data-science.md)

Elsewhere in the repository (these links work on GitHub and in a checkout; an installed copy of the manual has only its own folder):

- [The examples](../../examples/README.md), in the order to read them
- [CHANGELOG.md](../../CHANGELOG.md): what changed in each version
- [ROADMAP.md](../../ROADMAP.md): what comes next
- [LEGAL.md](../../LEGAL.md): what you may do with RapidR and the programs you build
- Where compatibility stands: the [compatibility suite on Windows](../rapidq-corpus-status.md) and [on macOS, Linux and the web](../rapidq-corpus-status-unix.md)
- The reference for every component and its members is
  [reference/](reference/components.md), in RapidR's own words.

The RapidR SDK installs this manual with its other documents: on Windows in
`share\doc\rapidr\manual` under the install folder
(`%LOCALAPPDATA%\Programs\RapidR`), on macOS inside the app
(`RapidR Studio.app/Contents/Resources/doc/manual`), on Linux in
`/usr/share/doc/rapidr/manual` (the `.deb`) or
`~/.local/share/doc/rapidr/manual` (the `.tar.gz`'s `install.sh`).

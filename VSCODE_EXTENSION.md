# RapidR for Visual Studio Code

The VS Code extension for RapidR BASIC, which is compatible with RapidQ (`.bas`, `.rr`, `.inc`, and RapidQ's own `.rqw`, `.rqb` and `.rq`, which it recognises too). Its source is in [`utilities/vscodeext/rapidr/`](utilities/vscodeext/rapidr/), and its Marketplace page is [the extension's README](utilities/vscodeext/rapidr/README.md).

RapidR is compatible with RapidQ and written from the ground up in pure Rust. It has a native compiler, an interpreter and a web runtime. The extension is a **thin client**: it holds no language knowledge of its own.

| What you see in VS Code | Where it comes from |
|---|---|
| Completion, hover, signature help, go to definition, references, rename, outline, formatting, diagnostics | `rapidr lsp`, the language server (the Language Server Protocol over stdio), built on `rapidr-langsvc`, the same engine as RapidR Studio |
| Breakpoints, stepping, call stack, variables, watches | `rapidr dap`, the debug adapter (the Debug Adapter Protocol over stdio) |
| Run, build and bundle commands | `rapidr run` / `build` / `bundle-bc`, in a terminal |
| Syntax highlighting, snippets, brackets, folding, indentation | static files in the extension: the TextMate grammar, `snippets/rapidr.json`, `language-configuration.json` |

VS Code and RapidR Studio therefore give the same answers, and both understand programs the way the compiler does. The diagnostics use RapidQ's compiler wording (`Undeclared identifier x`, `Member X not part of class Y`, …).

The extension's version is RapidR's version. Each RapidR release ships a matching `rapidr-<version>.vsix`.

## Installing

1. **Install RapidR** from the [GitHub releases](https://github.com/iBobX/RapidR/releases): the macOS `.dmg`, the Windows installer, or the Linux `.deb` / `.tar.gz`.
2. **Install the extension**, either:
   - from the VS Code Marketplace or Open VSX (search for "RapidR"), once it is published there (see [docs/vscode-publishing.md](docs/vscode-publishing.md)); or
   - from the release's `rapidr-<version>.vsix`: in VS Code, open the Extensions view, then **⋯ → Install from VSIX…**, or run `code --install-extension rapidr-<version>.vsix`.
3. Open a `.bas` or `.rr` file. The status bar shows **RapidR \<version\>** once the extension has found `rapidr`.

### How the extension finds `rapidr`

It lists every `rapidr` it can find, in this order:

1. The **`rapidr.path`** setting. This can name the executable, the folder it is in, an install prefix (with `bin/`), or `RapidR Studio.app` on macOS. If it is set and wrong, the extension says so and doesn't fall back to another `rapidr`.
2. The `RAPIDR_PATH` environment variable (for development and tests).
3. **`PATH`**, every `rapidr` on it in order. The extension searches it like `which` / `where` (with `PATHEXT` on Windows) and starts no process to do so.
4. The **install places**:
   - macOS: `/Applications/RapidR Studio.app/Contents/MacOS/rapidr`, `/Applications/RapidR Runtime.app/…` (or an older `RapidR.app`) and `~/Applications/…`, `/usr/local/bin`, `/opt/homebrew/bin`, `~/.local/bin`.
   - Linux: `/usr/bin/rapidr`, `/usr/local/bin/rapidr`, `~/.local/bin/rapidr`.
   - Windows: `%LOCALAPPDATA%\Programs\RapidR\bin\rapidr.exe`, `%ProgramFiles%\RapidR\bin\rapidr.exe`.
5. A **RapidR source checkout** open in the workspace: `./rapidr`, `target/release/rapidr`, `target/debug/rapidr`. This applies in trusted workspaces only.

A `rapidr` that is found isn't necessarily one that works. The extension runs `rapidr version` (it prints `RapidR <version>`) on the candidates in that order and takes **the first that is as new as the extension** (its major.minor: the extension's version is RapidR's). It stops asking as soon as it has one, so the usual machine runs one process.

- A `rapidr` **older than 2.117** has no language server and no debugger (the `rapidr` 0.1.0 that `cargo install` left in `~/.cargo/bin` is the usual one). It is skipped, a notification tells you once ("RapidR 0.1.0 at … is too old … Using RapidR 2.121.3 instead"), and the "RapidR Language Server" output says why.
- One **from 2.117 but older than the extension** has the language server and is used when nothing newer is found, with a warning in the status bar: newer features (Help in the editor, parameter names, colours) need a newer RapidR, and the extension says so when you ask for them.
- When **nothing usable** is found (or `rapidr.path` names one too old), a notification says what was found and offers **Use the installed RapidR** (clears `rapidr.path`, so the search finds it), **Choose…** (a file picker; saved in `rapidr.path`) and **Download** (the releases page). The status bar item shows a warning, and its menu has the same choices.

The status bar item (RapidR files only) shows **RapidR \<version\>** of the `rapidr` in use; its tooltip says which file.

## Features

- **Completion**: builtins, keywords, components (offered under RapidR's names, `RButton`, `RForm` …, each with "RapidQ name: QBUTTON" as its detail; in a RapidQ-compatible project they are offered under RapidQ's names, with "RapidR name: RButton"; both names are understood everywhere) and their properties, methods and events, and the program's own variables, constants, SUBs, FUNCTIONs and TYPEs, including those in `$INCLUDE` files (as the editor has them, saved or not). Member completion follows the variable's type, inside `WITH` and `CREATE` too. Completion triggers on `.`.
  When you type `AS ` (`DIM test AS `) the list of types and components opens by itself: the server's trigger characters are `.` and `$`, so the extension asks for the list when it sees the word AS finished with a space (not in a string or a comment, not when `editor.suggestOnTriggerCharacters` is off).
- **Hover** with signatures and documentation (a component shows as "RButton — RapidQ name: QBUTTON"), ending with **Open in RapidR Help**; and **signature help** (triggered by `(` and `,`).
- **Parameter names** (inlay hints, `textDocument/inlayHint`): the name of the parameter before a literal argument of a call of several (`DrawDot x, y: 40, r: 12`), the same chips Studio shows without Ctrl.
- **Colours** (`textDocument/documentColor`, `colorPresentation`): a swatch before `RGB(…)`, `&HBBGGRR` and the `cl…` constants, and VS Code's picker writing the colour back as the program wrote it. A system colour (`&H80000005`) has no swatch.
- **Go to Definition**, also into `$INCLUDE` files; **Find All References**; **Rename Symbol**; **document highlights** (every use of the name under the caret); an `$INCLUDE "file"` line is a link to the file (the extension's, not the server's).
- **Outline** and breadcrumbs: SUBs, FUNCTIONs, TYPEs, the CREATE tree. Folding comes from the language configuration (a regular expression per block, not the server).
- **Semantic highlighting** (`textDocument/semanticTokens/full`): functions, variables, parameters, properties, types and constants by what they are, over the TextMate grammar's colours.
- **Format Document**, **Format Selection** and format on type. The extension sets RapidR as the default formatter for RapidR files (`configurationDefaults` `[rapidr]` `editor.defaultFormatter`, the extension's own id `<publisher>.<name>`; a test fails if it and `package.json` disagree). What it does today: indentation of blocks and the case of keywords, types, directives and builtins; it leaves spacing inside a line (`x=1`, double spaces) and a directive's argument (`$APPTYPE console`) as they are.
- **Quick fixes** (`textDocument/codeAction`): the compiler's and the registry's (`NOT FILEEXISTS(x)` is always true → `FILEEXISTS(x) = 0`, a RapidQ include a constant needs, RapidR's name → RapidQ's in a RapidQ-compatible project).
- **Help** (RapidR's own request `rapidr/helpPage`, with `rapidr/helpSuggest` for the search box): the page Studio's Help pane shows, in a panel beside the editor — see below.
- **Diagnostics** as you type, in RapidQ's compiler wording. With `rapidr.rapidqCompatible` on, what RapidQ doesn't have is reported too: today RapidR's own components (`RPlot`, with no RapidQ counterpart) and RapidR's names of RapidQ's components (`RButton is RapidR's name: RapidQ's compiler knows it as QBUTTON`, with a quick fix to `QBUTTON`); members, builtins, statements and directives join when the language registry records each one's origin.
- **Run / Build / Bundle / Debug** commands, on the editor title bar, in the Command Palette, in the explorer's context menu, and in the status bar item's menu.
- Highlighting, snippets, folding and indentation for RapidR syntax (RapidQ-compatible programs included).

### Commands

| Command | What it runs |
|---|---|
| RapidR: Run File | `rapidr run <file>` in the "RapidR" terminal. It saves the file and the RapidR files it may `$INCLUDE` first. Running again ends the previous run. |
| RapidR: Debug File | Starts a `rapidr` debug session on the file. |
| RapidR: Build Native Executable | `rapidr build <file> --release`, which needs Rust (`rapidr setup`). |
| RapidR: Build Standalone Executable (Interpreted) | `rapidr build <file> --interp`: one executable, no Rust. |
| RapidR: Bundle for the Web (.zip) | `rapidr bundle-bc <file> -o <name>-web.zip`, written beside the file. |
| RapidR: Help for the Word at the Cursor | The Help panel (below), for the word at the caret. Also **Open in RapidR Help** in a hover. |
| RapidR: Run Project / Debug Project / Build Project | `rapidr run <main> <args>` (working folder `[run] cwd`), a debug session with the project's main file, arguments and folder, or `rapidr build <project.rrproj>`, for a `.rrproj` in the workspace (the explorer's or the editor's, else the one that holds the active file, else a pick). |
| RapidR: Open the Project's Main File | The `main` of the `.rrproj`. |
| RapidR: Restart Language Server | Finds `rapidr` again and restarts `rapidr lsp`. |
| RapidR: Show Language Server Output | The "RapidR Language Server" output channel. |
| RapidR: Locate rapidr… | Picks the executable and saves it in `rapidr.path`. |

The extension adds one keybinding, **Shift+F1** for Help in a RapidR editor (F1 is VS Code's Command Palette). F5 (debug) and Ctrl+F5 / Cmd+F5 (run without debugging) are VS Code's own and work on RapidR files through the debugger.

### Help in VS Code

`rapidr lsp` answers `rapidr/helpPage` with the page Studio's Help pane shows (the code is `rapidr_langsvc::help`, one implementation for both): Markdown whose `help:Word[@Type]` links open another entry, `help-insert:` types the syntax at the editor's caret, `help-goto:` jumps to where one of the program's own names is declared and `help-refs:` finds its references. The extension shows it in a webview panel beside the editor with a search box (its suggestions come from `rapidr/helpSuggest`), a Back button, and the page's links. The page is rendered by the extension's own small Markdown reader (`src/markdown.js`): everything is escaped, only `help:` and `https:` links are followed, and the panel's script runs under a nonce-only Content-Security-Policy. A server without `rapidr/helpPage` (older than the extension) says so when Help is asked for.

### Projects

A project is a `.rrproj` (TOML, written by Studio; [Projects](docs/manual/studio-projects.md)). The extension reads `name`, `main`, `[run] args` and `cwd` with a small reader of the TOML subset the format uses (`src/project.js`: no package, unknown keys ignored, as RapidR does). `.rrproj` files open as the language "RapidR Project", highlighted.

### Settings

| Setting | Default | |
|---|---|---|
| `rapidr.path` | `""` | The `rapidr` to use (machine-overridable). |
| `rapidr.rapidqCompatible` | `false` | Report what RapidQ doesn't have. The server receives this as the `rapidqCompatible` initialization option, and changing it restarts the server. |
| `rapidr.trace.server` | `off` | `messages` / `verbose`: log the LSP traffic in the output channel. |

## Debugging

The extension contributes the **`rapidr`** debug type. VS Code runs `rapidr dap` as the debug adapter, over stdio. With no `launch.json`, F5 debugs the active RapidR file (`program: ${file}`). To give arguments, a working folder or environment variables, add a configuration (**Run → Add Configuration… → RapidR**):

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "rapidr",
      "request": "launch",
      "name": "RapidR: main.bas",
      "program": "${workspaceFolder}/main.bas",
      "args": ["data.csv"],
      "cwd": "${workspaceFolder}",
      "stopOnEntry": false,
      "env": { "MY_SETTING": "1" }
    }
  ]
}
```

| Attribute | |
|---|---|
| `program` | The `.bas`, `.rr` or `.rrbc` to run. Default: `${file}`. |
| `args` | The program's command line (`COMMAND$(n)`). |
| `cwd` | The program's folder (default: the program's own folder). |
| `stopOnEntry` | Pause on the first statement. |
| `noDebug` | Run without debugging. Ctrl+F5 sets this. |
| `env` | Extra environment variables (`null` removes one). |

Conditional breakpoints (`i = 2`), hit counts (`3`, `>= 3`, `% 3`) and logpoints (`adding {i} to {total}`) work: the adapter advertises them and the program session evaluates them.

The program runs as `rapidr run --session` (RapidR's program session protocol; RapidR Studio's debugger speaks it too). While it is stopped:

- **Watches, hovers and the Variables view** evaluate BASIC expressions in the selected frame on the VM itself (`sum / n`, `UCASE$(name$)`, `values(k) * 2`, a component's property `Form.Caption`); arrays and TYPEs expand.
- **The Debug Console** prints an expression (`total * 2`) and runs a statement (`total = 100`, `PRINT x`), as VB's Immediate window does. While the program runs, a line typed there is its `INPUT`.
- **Set Value** in the Variables view takes any expression.
- **Run-time errors** (the Breakpoints view's "Run-time errors"): the program stops at the faulting statement — in an event handler too — with its frames and locals.
- Breakpoints work in any file of the program (`$INCLUDE` files included); pause stops a busy program at its next statement.

## Troubleshooting

- **"RapidR was not found."** Use **Locate rapidr…**, or set `rapidr.path`. VS Code started from the macOS Dock may have a shorter `PATH` than your shell. The install places are searched anyway.
- **"The RapidR found is too old for this extension."** A `rapidr` older than 2.117 has no language server and no debugger (a 0.1.0 from `cargo install` in `~/.cargo/bin`). The extension skips it when a newer one is installed (it tells you once); otherwise pick **Use the installed RapidR**, **Choose…** or **Download**. In a terminal, `which -a rapidr` and `rapidr version` show what your shell finds.
- **No completion, hover or debugging, and the status bar says nothing.** The status bar item shows only on RapidR files; open one. **RapidR: Show Language Server Output** lists every `rapidr` the extension looked at, the version it asked, and which it used.
- **"RapidR's language server didn't start."** Run **RapidR: Show Language Server Output**. A RapidR older than the language server has no `rapidr lsp`, so update it. To see the protocol traffic, set `rapidr.trace.server` to `verbose`.
- **Nothing happens on F5.** The active editor must be a RapidR file, or `launch.json` must name a `program`.
- **Untrusted workspace.** The workspace's `rapidr.path` is ignored, and a `rapidr` built inside the workspace isn't run, until you trust the workspace.
- **`.bas` opens as another language.** Another extension claims `.bas` (Visual Basic, for example). Choose "RapidR" in the language picker on the status bar, or add `"files.associations": { "*.bas": "rapidr" }`.

## Building the extension

It is plain JavaScript (no TypeScript), bundled with esbuild into `dist/extension.js`. The `.vsix` therefore carries one script and no `node_modules`. Building it needs Node.js 22 or newer.

```bash
./build_vsc_extension.sh            # → utilities/vscodeext/rapidr/dist/rapidr-<version>.vsix
```

That script runs `npm ci` and `npm run package` in `utilities/vscodeext/rapidr/`. The package step does three things:

- `scripts/sync-version.js` copies RapidR's version (Cargo.toml's `[workspace.package] version`) into `package.json`.
- `vscode:prepublish` bundles the extension (`scripts/build.js --production`) and regenerates `THIRD_PARTY_NOTICES.md` (`scripts/notices.js`). The notices list every bundled JavaScript package with its licence text, and the step fails on a licence other than MIT, ISC, BSD, Apache-2.0 or 0BSD.
- `vsce package --no-dependencies` makes the `.vsix`.

`npm run ls` lists what goes into the `.vsix`. `.vscodeignore` keeps only the bundle, the grammar, snippets, language configuration, images, README, CHANGELOG, LICENSE and the notices.

The release puts the `.vsix` beside the installers: `tools/release/vscode.sh` (run by `prepare.sh`) writes `dist/<version>/out/rapidr-<version>.vsix`, and `finish.sh`'s `SHA256SUMS` covers it. Publishing to the Marketplace and Open VSX is done by hand, with your own accounts: see [docs/vscode-publishing.md](docs/vscode-publishing.md).

| Production dependency | Licence |
|---|---|
| `vscode-languageclient` (with `vscode-languageserver-protocol`, `vscode-jsonrpc`, `vscode-languageserver-types`) | MIT |
| `minimatch`, `semver` | ISC |
| `brace-expansion`, `balanced-match` | MIT |

Development only: `esbuild`, `@vscode/vsce`, `@vscode/test-electron` and `mocha`, all MIT.

## Testing the extension

```bash
cd utilities/vscodeext/rapidr
npm install
cargo build -p rapidr-cli                       # (from the repository root) a rapidr with `lsp`
RAPIDR_PATH=/path/to/rapidr npm run test:unit   # plain Node: finding rapidr, versions, Help's Markdown, the grammar, the snippets
npm test                                        # unit tests, then the integration tests
RAPIDR_PATH=/path/to/rapidr npm test            # another rapidr
RAPIDR_TEST_GREP=diagnostics npm test           # some tests only
VSCODE_VERSION=1.90.0 npm test                  # the oldest VS Code supported
VSCODE_PATH="/Applications/Visual Studio Code.app/Contents/MacOS/Code" npm test   # an installed VS Code (still a throwaway profile)
npm run grammar:check                           # the grammar has every name of the registry (npm run grammar adds what is missing)
```

The unit tests (`test/unit/`) need no VS Code. The registry tests (the grammar, the snippets) need a `rapidr` (`RAPIDR_PATH`, or `target/debug/rapidr`) and are skipped without one.

The integration tests (`test/runTest.js`, `test/suite/`) use `@vscode/test-electron`. They start a VS Code with a temporary profile, never yours, and only this extension (`VSCODE_PATH` uses one already installed; else one is downloaded once into `.vscode-test/`). That VS Code opens `test/fixtures/workspace/`; the tests that write files write them in a temporary folder, and the fixtures are reverted. It runs twice: the suite, then `test/suite/path.test.js` in a VS Code whose `PATH` has a `rapidr` 0.1.0 first and the real one after it (the bug of 2.121.3).

The tests ask VS Code what its own UI asks (`vscode.executeCompletionItemProvider`, `…HoverProvider`, `…SignatureHelpProvider`, `…DefinitionProvider`, `…ReferenceProvider`, `…DocumentRenameProvider`, `…DocumentSymbolProvider`, `…FormatDocumentProvider`, `…InlayHintProvider`, `…DocumentColorProvider`, `…ColorPresentationProvider`, `…DocumentHighlights`, `…LinkProvider`, `…CodeActionProvider`, `languages.getDiagnostics`) and drive the commands. They check:

- **Finding rapidr** (`features.test.js`, `path.test.js`): the version and state are known; a rapidr 0.1.0 in `rapidr.path` is refused and the real one offered; one older than the extension is used and says so; with the ancient one first on `PATH` the real one is used, and completion, hover and debugging work.
- **The language server** (`extension.test.js`): completion after `Form.`, builtins, the program's names; hover and signature help; definition into an include, references, rename, outline; formatting changes only layout and letter case; diagnostics in RapidQ's words, an include as the editor has it; the registry's words; automatic keyword case.
- **Formatting and typing** (`features.test.js`): RapidR is the default formatter (and its id is `<publisher>.<name>`), Format Document and format on save format, the types are listed after `DIM test AS ` and the list opens by itself when the space after AS is typed.
- **Hints, colours, highlights, quick fixes, links**: the parameter names, the swatches and the picker's forms, the highlights, the fix for `NOT FILEEXISTS`, the `$INCLUDE` link.
- **Help**: the page of a word, of a property in a CREATE body, of a name of the program (and Go to Declaration), links, Back, search, Insert into the code, and nothing but help and https links followed.
- **Projects**: a `.rrproj` is a RapidR project; Open the Project's Main File; Debug Project passes the main file, the project's arguments and folder; Run Project opens the "RapidR" terminal.
- **Debugging** (`extension.test.js`, `debug.test.js`): a breakpoint, the call stack, locals and globals, watches, the Debug Console; conditions, hit counts, logpoints, step in / over / out, `stopOnEntry`, the launch's arguments, environment and folder, run without debugging.
- **Contributions and commands**: the debug type, breakpoints, launch attributes, the commands, Run File and its terminal.

The tests run as a normal window that opens briefly. On Linux without a display, use `xvfb-run -a npm test`.

## The screenshots

`test/screenshots/capture.js` takes the README's pictures (`images/*.png`) from a real VS Code driven through each feature (`driver.js`), on the programs in `test/screenshots/showcase/`; `SHOTS_DIR=… node test/screenshots/capture.js` keeps every state (definition, rename, the registry's warnings, a SUB's own variables, the list of types after `AS `, Help, the commands), and `SHOTS_SCENARIO=old-path` takes the window of a user whose `PATH` has the ancient `rapidr` first. macOS only (`screencapture` of the window, not the screen; the terminal needs the Screen Recording permission). Look at every image before committing.

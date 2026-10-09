# RapidR for Visual Studio Code

The VS Code extension for RapidR and RapidQ BASIC (`.bas`, `.rr`, `.inc`). Its source is in [`utilities/vscodeext/rapidr/`](utilities/vscodeext/rapidr/), and its Marketplace page is [the extension's README](utilities/vscodeext/rapidr/README.md).

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

It checks these places in order:

1. The **`rapidr.path`** setting. This can name the executable, the folder it is in, an install prefix (with `bin/`), or `RapidR Studio.app` on macOS. If it is set and wrong, the extension says so and doesn't fall back to another `rapidr`.
2. The `RAPIDR_PATH` environment variable (for development and tests).
3. **`PATH`**. The extension searches it like `which` / `where` (with `PATHEXT` on Windows) and starts no process to do so.
4. The **install places**:
   - macOS: `/Applications/RapidR Studio.app/Contents/MacOS/rapidr` (or an older `RapidR.app`) and `~/Applications/…`, `/usr/local/bin`, `/opt/homebrew/bin`, `~/.local/bin`.
   - Linux: `/usr/bin/rapidr`, `/usr/local/bin/rapidr`, `~/.local/bin/rapidr`.
   - Windows: `%LOCALAPPDATA%\Programs\RapidR\bin\rapidr.exe`, `%ProgramFiles%\RapidR\bin\rapidr.exe`.
5. A **RapidR source checkout** open in the workspace: `./rapidr`, `target/release/rapidr`, `target/debug/rapidr`. This applies in trusted workspaces only.

The extension then runs `rapidr version`, which prints `RapidR <version>`. If `rapidr` is not found, a notification says **"RapidR was not found"** and offers two buttons. **Download RapidR** opens the releases page. **Locate rapidr…** opens a file picker and saves your choice in `rapidr.path`. The status bar item then shows a warning.

## Features

- **Completion**: builtins, keywords, components (RapidQ's offered under their `Q` names, RapidR's own under `R` names; both names understood everywhere) and their properties, methods and events, and the program's own variables, constants, SUBs, FUNCTIONs and TYPEs, including those in `$INCLUDE` files (as the editor has them, saved or not). Member completion follows the variable's type, inside `WITH` and `CREATE` too. Completion triggers on `.`.
- **Hover** with signatures and documentation, and **signature help** (triggered by `(` and `,`).
- **Go to Definition**, also into `$INCLUDE` files; **Find All References**; **Rename Symbol**.
- **Outline** and breadcrumbs: SUBs, FUNCTIONs, TYPEs, the CREATE tree.
- **Format Document**.
- **Diagnostics** as you type, in RapidQ's compiler wording. With `rapidr.rapidqCompatible` on, what RapidQ doesn't have is reported too: today RapidR's own components and Q-names RapidQ lacks (`QPLOT`, with a quick fix to `RPlot`); members, builtins, statements and directives join when the language registry records each one's origin.
- **Run / Build / Bundle / Debug** commands, on the editor title bar, in the Command Palette, in the explorer's context menu, and in the status bar item's menu.
- Highlighting, snippets, folding and indentation for RapidQ / RapidR syntax.

### Commands

| Command | What it runs |
|---|---|
| RapidR: Run File | `rapidr run <file>` in the "RapidR" terminal. It saves the file and the RapidR files it may `$INCLUDE` first. Running again ends the previous run. |
| RapidR: Debug File | Starts a `rapidr` debug session on the file. |
| RapidR: Build Native Executable | `rapidr build <file> --release`, which needs Rust (`rapidr setup`). |
| RapidR: Build Standalone Executable (Interpreted) | `rapidr build <file> --interp`: one executable, no Rust. |
| RapidR: Bundle for the Web (.zip) | `rapidr bundle-bc <file> -o <name>-web.zip`, written beside the file. |
| RapidR: Restart Language Server | Finds `rapidr` again and restarts `rapidr lsp`. |
| RapidR: Show Language Server Output | The "RapidR Language Server" output channel. |
| RapidR: Locate rapidr… | Picks the executable and saves it in `rapidr.path`. |

The extension adds no keybindings. F5 (debug) and Ctrl+F5 / Cmd+F5 (run without debugging) are VS Code's own and work on RapidR files through the debugger.

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

The program runs as `rapidr run --session` (RapidR's program session protocol; RapidR Studio's debugger speaks it too). While it is stopped:

- **Watches, hovers and the Variables view** evaluate BASIC expressions in the selected frame on the VM itself (`sum / n`, `UCASE$(name$)`, `values(k) * 2`, a component's property `Form.Caption`); arrays and TYPEs expand.
- **The Debug Console** prints an expression (`total * 2`) and runs a statement (`total = 100`, `PRINT x`), as VB's Immediate window does. While the program runs, a line typed there is its `INPUT`.
- **Set Value** in the Variables view takes any expression.
- **Run-time errors** (the Breakpoints view's "Run-time errors"): the program stops at the faulting statement — in an event handler too — with its frames and locals.
- Breakpoints work in any file of the program (`$INCLUDE` files included); pause stops a busy program at its next statement.

## Troubleshooting

- **"RapidR was not found."** Use **Locate rapidr…**, or set `rapidr.path`. VS Code started from the macOS Dock may have a shorter `PATH` than your shell. The install places are searched anyway.
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
npm run test:unit                               # finding rapidr, terminal quoting: plain Node
cargo build -p rapidr-cli                       # (from the repository root) a rapidr with `lsp`
npm test                                        # unit tests, then the integration tests
RAPIDR_PATH=/path/to/rapidr npm test            # another rapidr
RAPIDR_TEST_GREP=diagnostics npm test           # some tests only
VSCODE_VERSION=1.90.0 npm test                  # the oldest VS Code supported
```

The integration tests (`test/runTest.js`, `test/suite/`) use `@vscode/test-electron`. They download a VS Code once into `.vscode-test/` and start it with a temporary profile, never yours, and only this extension. That VS Code opens `test/fixtures/workspace/`, which holds four files:

- `main.bas`: a form with a button, a SUB, and an `$INCLUDE`.
- `helpers.inc`: a FUNCTION.
- `errors.bas`: an undeclared variable under `$TYPECHECK ON`.
- `hello.bas`: a console program for the Run command.

The tests ask VS Code what its own UI asks (`vscode.executeCompletionItemProvider`, `…HoverProvider`, `…SignatureHelpProvider`, `…DefinitionProvider`, `…ReferenceProvider`, `…DocumentRenameProvider`, `…DocumentSymbolProvider`, `…FormatDocumentProvider`, `languages.getDiagnostics`). They check six things:

- Completion offers the members after `Form.` (`Caption`), builtins (`MID$`), the program's SUB, variable, and the FUNCTION from the include.
- Hover and signature help name what is under the cursor.
- Definition goes into `helpers.inc`, references and rename find all four uses, and the outline has the SUB, the form and its button.
- Formatting changes only layout and letter case, and the error is reported as `Undeclared identifier undeclaredThing` on its line.
- The `rapidr` debug type, its breakpoints and its launch attributes are contributed.
- Run File opens the "RapidR" terminal, and running again replaces it.

The tests run on macOS as a normal window that opens briefly. On Linux without a display, use `xvfb-run -a npm test`.

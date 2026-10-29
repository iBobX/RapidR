# RapidR in other editors

RapidR Studio is the editor made for RapidR, but a program is a text file, and two programs that come with RapidR let any editor that speaks the standard protocols understand it the way Studio does:

- **`rapidr lsp`** is the language server (the Language Server Protocol, over standard input and output). It is the engine of Studio's code editor, so another editor gets the same completion, hover help, parameter help, Go to Definition, references, rename, outline, formatting, quick fixes, and the same problems worded the way RapidQ's compiler words them.
- **`rapidr dap`** is the debug adapter (the Debug Adapter Protocol), the engine behind Studio's debugger: breakpoints, stepping, the call stack, variables, watches.

There is a ready-made RapidR extension for Visual Studio Code. This chapter says what it does and how it finds RapidR, then what another editor can ask the server for.

## The Visual Studio Code extension

Install RapidR first (the [download page](https://github.com/iBobX/RapidR/releases)), then the extension, `rapidr-<version>.vsix` from the same release (**Extensions ▸ ⋯ ▸ Install from VSIX…**). Open a `.bas` or `.rr` file: the status bar shows **RapidR 2.121.3** (the version in use). The extension holds no language knowledge of its own; everything below is RapidR's.

### Which `rapidr` it uses

The extension looks in `rapidr.path` (if you set it), then on `PATH`, then where RapidR's installers put it (`/Applications/RapidR Studio.app` on macOS, the Program Files or `%LOCALAPPDATA%\Programs\RapidR` folder on Windows, `/usr/bin` on Linux), and asks each its version. It uses **the first that is as new as the extension**. A `rapidr` older than 2.117 has no language server or debugger (the `rapidr` 0.1.0 that `cargo install` left behind is the one people meet): the extension skips it, tells you once, and uses the installed RapidR instead. The **RapidR Language Server** output lists each one it looked at and why it used or skipped it. If nothing is new enough, a notification offers **Use the installed RapidR**, **Choose…** (a file picker) and **Download**.

### What you get

- **Completion** of keywords, builtins, components (under RapidR's names) with their properties, methods and events, and your own names, `$INCLUDE` files included. Type `DIM test AS ` and the list of types and components opens by itself.

  ![The list of types after AS](../../utilities/vscodeext/rapidr/images/completion-as.png)

- **Hover** and **signature help**, with the parameter's name written before each literal argument. The hover ends with **Open in RapidR Help**.
- **RapidR Help beside your code**: **Shift+F1** on a word opens the page Studio's Help pane shows for it — what it is, the syntax, what it does, a component's properties, methods and events as links, your own names with where they are declared. *Insert into the code*, *Back* and a search box are on the page.

  ![RapidR Help beside the code](../../utilities/vscodeext/rapidr/images/help.png)

- **Colour swatches** before `RGB(…)`, `&HBBGGRR` and the `cl…` names, with the editor's colour picker writing the colour back the way the program wrote it.

  ![Parameter names and colours](../../utilities/vscodeext/rapidr/images/hints.png)

- **Go to Definition**, **Find All References**, **Rename**, the **Outline**, the highlights of the name under the caret, quick fixes, and Format Document (RapidR is the default formatter for RapidR files; turn on `"editor.formatOnSave"` for the RapidR language to format when you save).
- **Run, Debug, Build** commands, and for a project (`.rrproj`) **Run Project**, **Debug Project**, **Build Project** and **Open the Project's Main File**, using the project's main file, arguments and folder.
- **Debugging** with F5: breakpoints (with conditions, hit counts and log messages), stepping, the call stack, Locals, Globals and the form's Components, watches, and the Debug Console, where an expression is printed and a statement runs.

  ![Debugging in the editor](../../utilities/vscodeext/rapidr/images/debug.png)

The extension's own page, with its settings (`rapidr.path`, `rapidr.rapidqCompatible`, `rapidr.keywordCase`, `rapidr.identifierCase`) and its troubleshooting, is its [README](../../utilities/vscodeext/rapidr/README.md); how it is built and tested is [VSCODE_EXTENSION.md](../../VSCODE_EXTENSION.md).

## Any editor that speaks LSP

`rapidr lsp` takes these **initialization options** (all optional): `rapidqCompatible` (true: report what RapidQ doesn't have), `keywordCase` (`upper`, `lower`, `proper`, `preserve`), `identifierCase` (`declaration`, `preserve`) and `includeDirs` (more folders for `$INCLUDE`). It reads files only inside the workspace folders the editor opened and the folders of the documents it opened.

| What | Request |
|---|---|
| Completion (trigger characters `.` and `$`), hover, signature help | `textDocument/completion`, `hover`, `signatureHelp` |
| Go to Definition, references, highlights, rename (with prepare) | `definition`, `references`, `documentHighlight`, `rename` |
| Outline | `documentSymbol` |
| Formatting, range and on type (the keyword case) | `formatting`, `rangeFormatting`, `onTypeFormatting` |
| Quick fixes | `codeAction` |
| Names by what they are | `semanticTokens/full` |
| Parameter names | `inlayHint` |
| Colours | `documentColor`, `colorPresentation` |
| Problems | `publishDiagnostics` (a main file's, and those of its `$INCLUDE` files) |
| RapidR's Help page of a word or of the word at a position | `rapidr/helpPage` (`{ word?, of?, textDocument?, position?, canInsert?, key? }` → `{ kind, markdown, insert, uri, line, column }`) |
| What a typed word may mean | `rapidr/helpSuggest` |

`rapidr/helpPage`'s `markdown` is the page Studio shows; its links are `help:Word[@Type]` (another entry), `help-insert:` (type `insert` at the caret), `help-goto:` (open `uri` at `line`, `column`) and `help-refs:` (find references). The server says it has them in `capabilities.experimental.rapidr`.

What the server doesn't have: workspace symbols, folding ranges (the extension uses regular expressions), selection ranges, code lenses and call hierarchy.

`rapidr dap` is launched by the editor as the debug adapter: `{ "type": "rapidr", "request": "launch", "program": "main.bas", "args": [], "cwd": ".", "stopOnEntry": false, "noDebug": false, "env": {} }`. It supports conditional breakpoints, hit counts, logpoints, the run-time error filter, set variable and set expression (any BASIC expression), step in targets, and going to a line (`goto`).

## If it doesn't work

- **The status bar says RapidR is too old, or no completion appears.** Run `rapidr version` in a terminal (`which -a rapidr` shows every one on your `PATH`), then **RapidR: Show Language Server Output** in the editor.
- **Help says this `rapidr` has no Help pages.** The extension is newer than the `rapidr` it found: update RapidR.

Reproduce the pictures: `node test/screenshots/capture.js` in `utilities/vscodeext/rapidr` (macOS).

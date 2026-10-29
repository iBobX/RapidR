# Changelog

The extension's version is the version of RapidR it ships with.

## Unreleased

Robert's first walk in 2.121.3 found that an ancient `rapidr` 0.1.0 (from `cargo install`) first on `PATH` was used, so there was no completion, no hover, and debugging did nothing, with no word why. This is the fix, and a lot more.

- **Finding RapidR, properly.** The extension asks every `rapidr` it finds (`PATH`, the installed RapidR, a source checkout) for its version and uses the first that is as new as the extension. One too old for the language server (before 2.117) is skipped, and said so once; one older than the extension but with the language server is used, with a warning in the status bar. When none will do, a notification offers **Use the installed RapidR**, **Choose…** and **Download**. `rapidr.path` is still never replaced silently. The output channel lists every `rapidr` it looked at. macOS: `RapidR Runtime.app` too; Windows: the Program Files installs too.
- **RapidR is the default formatter** for RapidR files (`editor.defaultFormatter`), so Format Document asks nothing; format on save works with `editor.formatOnSave`.
- **RapidR Help beside your code** (Shift+F1, "Open in RapidR Help" in a hover): the page Studio's Help shows, with a search box, Back, Insert into the code, and for your own names where they are declared and Find References.
- **Parameter names** before literal arguments, **colour swatches** with VS Code's picker (written back as `RGB(…)` or `&HBBGGRR`, as the program wrote them), and every use of the name under the caret highlighted.
- **The list of types opens by itself** when you type `DIM test AS `.
- **Projects:** Run, Debug, Build Project and Open the Project's Main File for a `.rrproj` (its main file, arguments and folder); `.rrproj` files are highlighted. An `$INCLUDE "file"` line is a link to the file.
- **Debugging:** conditional breakpoints, hit counts and logpoints are offered (the debug adapter now says it has them).
- **Grammar and snippets:** the grammar's word lists were 108 names behind RapidR's registry; `npm run grammar` brings them in step and a test fails when they drift. Every snippet is checked against the language server (the `fors` snippet had a loop that didn't compile as typed).
- Hover on a type after `AS` says the type (it said `STRING$` for `AS STRING`); a property set in a CREATE body (`Caption = "Hi"`) belongs to its component in Help.
- Tests: the integration suite is 57 tests (finding rapidr, the formatter, typing, hints, colours, Help, projects, debugging with conditions, logpoints and stepping) plus one in a VS Code whose `PATH` has the ancient `rapidr` first.
- **Automatic keyword case** as you type (`rapidr.keywordCase`: upper, lower, proper, preserve) and your names as declared (`rapidr.identifierCase`); Format Document and the new Format Selection apply them too. Format on type is on by default for RapidR files.
- Hover, completion and the warnings now come from RapidR's language registry: types, defaults and the include file of RapidQ's names; what RapidR doesn't have yet; what works on one runtime only; in RapidQ-compatible mode, every RapidR extension (members, builtins, statements, directives, types), not only components.
- The debugger shows a SUB's STATIC variables and its own variables in its Locals, under their names; Globals shows only globals.

## 2.117.0

Rebuilt on RapidR's language server and debug adapter.

- **Code completion from `rapidr lsp`.** The language server in RapidR (Rust) gives completion of builtins, keywords, components and their members, and your own variables, SUBs, FUNCTIONs and TYPEs. It also gives hover, signature help, go to definition (also into `$INCLUDE` files), find references, rename, the outline, formatting, and diagnostics worded as RapidQ's compiler words them. These are the same answers RapidR Studio gives. The regular-expression providers and checks of earlier versions are gone.
- **Debugging through `rapidr dap`.** This adds the `rapidr` debug type, with breakpoints, stepping, call stack, variables and watches. F5 debugs the active file without a `launch.json`.
- **RapidQ-compatible mode** (`rapidr.rapidqCompatible`): warns about everything RapidQ doesn't have.
- **Commands:** Run File (in the "RapidR" terminal), Debug File, Build Native Executable, Build Standalone Executable (interpreted), Bundle for the Web, Restart Language Server, Show Language Server Output, Locate rapidr…. Run and Debug are on the editor title bar. A status bar item shows the RapidR version and lists the commands.
- **Finding RapidR:** the `rapidr.path` setting, then `PATH`, then the usual install places, then a source checkout in the workspace. If RapidR isn't found, the extension offers to download it or to locate it.
- **Files:** `.bas` and `.inc` open as RapidR, as well as `.rr`. The grammar knows every component under its `Q` and `R` name and the current directives. Stale snippets were fixed.
- **Packaging:** one bundled file (esbuild). The bundled packages' licences are in THIRD_PARTY_NOTICES.md.

## 2.7.0 and earlier

Regular-expression completion, hover, signature help and outline; syntax highlighting; snippets; build commands.

# Changelog

The extension's version is the version of RapidR it ships with.

## Unreleased

- **Automatic keyword case** as you type (`rapidr.keywordCase`: upper, lower, proper, preserve) and your names as declared (`rapidr.identifierCase`); Format Document and the new Format Selection apply them too. Format on type is on by default for RapidR files.
- Hover, completion and the warnings now come from RapidR's language registry: types, defaults and the include file of RapidQ's names; what RapidR doesn't have yet; what works on one runtime only; in RapidQ-compatible mode, every RapidR extension (members, builtins, statements, directives, types), not only components.
- The debugger shows a SUB's STATIC variables and its own variables in its Locals, under their names; Globals shows only globals.

## 2.117.0

Rebuilt on RapidR's language server and debug adapter.

- **IntelliSense from `rapidr lsp`.** The language server in RapidR (Rust) gives completion of builtins, keywords, components and their members, and your own variables, SUBs, FUNCTIONs and TYPEs. It also gives hover, signature help, go to definition (also into `$INCLUDE` files), find references, rename, the outline, formatting, and diagnostics worded as RapidQ's compiler words them. These are the same answers RapidR Studio gives. The regular-expression providers and checks of earlier versions are gone.
- **Debugging through `rapidr dap`.** This adds the `rapidr` debug type, with breakpoints, stepping, call stack, variables and watches. F5 debugs the active file without a `launch.json`.
- **RapidQ-compatible mode** (`rapidr.rapidqCompatible`): warns about everything RapidQ doesn't have.
- **Commands:** Run File (in the "RapidR" terminal), Debug File, Build Native Executable, Build Standalone Executable (interpreted), Bundle for the Web, Restart Language Server, Show Language Server Output, Locate rapidr…. Run and Debug are on the editor title bar. A status bar item shows the RapidR version and lists the commands.
- **Finding RapidR:** the `rapidr.path` setting, then `PATH`, then the usual install places, then a source checkout in the workspace. If RapidR isn't found, the extension offers to download it or to locate it.
- **Files:** `.bas` and `.inc` open as RapidR, as well as `.rr`. The grammar knows every component under its `Q` and `R` name and the current directives. Stale snippets were fixed.
- **Packaging:** one bundled file (esbuild). The bundled packages' licences are in THIRD_PARTY_NOTICES.md.

## 2.7.0 and earlier

Regular-expression completion, hover, signature help and outline; syntax highlighting; snippets; build commands.

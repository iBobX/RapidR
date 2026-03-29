# RapidP — VS Code Extension

> The official Visual Studio Code extension for RapidP, providing a complete development environment for the RapidP programming language.

---

## Why This Extension Exists

**RapidP** is a modern BASIC-to-Python transpiler that lets you write desktop GUI applications, console tools, database programs, and network clients/servers using a clean, familiar BASIC syntax. The compiler translates `.rp` source files into Python, which can then be run directly or packaged as standalone executables.

Without editor support, writing RapidP means working in a plain text file with no visual feedback — no colour-coded keywords, no autocomplete for the 40+ built-in GUI components, no way to quickly check a function's signature, and no one-click compilation. Every property name, event handler, and function call has to be typed from memory or looked up in the manual.

**This extension solves all of that.** It turns VS Code into a first-class RapidP IDE where you can write, understand, navigate, and compile RapidP programs without ever leaving the editor.

---

## What It Does

At a high level, the extension provides six integrated capabilities:

1. **Syntax Highlighting** — A full TextMate grammar that colours every element of the language so you can read code at a glance.
2. **IntelliSense & Autocomplete** — Context-aware completions that know which component you're working with and suggest the right properties, methods, and events.
3. **Hover Documentation** — Instant reference docs for every keyword, function, component type, and directive, displayed right where your cursor is.
4. **Signature Help** — Parameter hints that appear as you type function arguments, showing what each parameter expects.
5. **Document Symbols & Outline** — A structural overview of your file (SUBs, FUNCTIONs, TYPEs, CREATE blocks) in the sidebar and breadcrumbs.
6. **Compile & Run Integration** — One-key compilation and execution from inside VS Code, including standalone executable builds.

---

## Features in Detail

### Syntax Highlighting

The extension registers a comprehensive TextMate grammar (`syntaxes/rapidp.tmLanguage.json`) that applies accurate scopes to every language construct. VS Code's theme engine then colours them according to whatever colour theme you use (Dark+, Monokai, Solarized, etc.).

What gets highlighted:

| Element | Examples |
|---------|----------|
| **Keywords** | `DIM`, `IF`, `THEN`, `ELSE`, `ELSEIF`, `FOR`, `TO`, `STEP`, `NEXT`, `WHILE`, `WEND`, `DO`, `LOOP`, `UNTIL`, `SUB`, `FUNCTION`, `SELECT CASE`, `CREATE`, `WITH`, `TYPE`, `IMPORT`, `DECLARE`, `BIND`, `CONST`, `RETURN`, `EXIT`, `CALL`, `GOTO`, `GOSUB` |
| **Data types** | `INTEGER`, `STRING`, `DOUBLE`, `SINGLE`, `BYTE`, `WORD`, `DWORD`, `LONG`, `INT64`, `CURRENCY`, `POBJECT`, `VARIANT` |
| **Component types** | `PFORM`, `PBUTTON`, `PLABEL`, `PEDIT`, `PCANVAS`, `PPANEL`, `PCHECKBOX`, `PRADIOBUTTON`, `PCOMBOBOX`, `PLISTBOX`, `PRICHEDIT`, `PTIMER`, `PPROGRESSBAR`, `PSTRINGGRID`, `PTABCONTROL`, `PCODEEDITOR`, `PIMAGE`, `PLISTVIEW`, `PTREEVIEW`, `PSCROLLBAR`, `PSTATUSBAR`, `PMAINMENU`, `PMENUITEM`, `PPOPUPMENU`, `PMYSQL`, `PSQLITE`, `PSOCKET`, `PSERVERSOCKET`, `PHTTP`, `PFILESTREAM`, `POPENDIALOG`, `PSAVEDIALOG`, `PCOLORDIALOG`, `PFONTDIALOG`, `PDESIGNSURFACE`, `PNUMPY`, `PMATPLOTLIB`, `PPANDAS`, and more |
| **Built-in functions** | 100+ functions across string, math, I/O, system, GUI, array, and conversion categories |
| **Literals** | Strings (`"..."`), decimal numbers, hex (`&HFF`), octal (`&O77`), binary (`&B1010`) |
| **Directives** | `$APPTYPE`, `$INCLUDE`, `$DEFINE`, `$UNDEF`, `$IFDEF`, `$IFNDEF`, `$ELSE`, `$ENDIF`, `$MACRO`, `$TYPECHECK`, `$OPTION`, `$OPTIMIZE`, `$ESCAPECHARS` |
| **Comments** | Single-quote (`'`) line comments and `REM` statements |
| **Operators** | `AND`, `OR`, `NOT`, `XOR`, `MOD`, `+`, `-`, `*`, `/`, `\`, `^`, `=`, `<>`, `<`, `>`, `<=`, `>=` |

### IntelliSense & Autocomplete

The completion engine (`src/completionProvider.js`) analyses the cursor context in real time and provides different suggestions depending on where you are in the code:

- **Dot completion** — Type a variable name followed by `.` and the extension looks up the variable's type (from its `CREATE` or `DIM` statement), then shows only the members that belong to that specific type. This works for both **built-in GUI components** and **user-defined TYPEs**.
  - *Built-in components:* typing `myButton.` on a `PBUTTON` variable shows `caption`, `width`, `onclick`, `setfocus`, etc.
  - *User-defined TYPEs:* typing `r.` on a `DIM r AS Rect` variable (where `Rect` is a TYPE you defined) shows all of `Rect`'s fields, SUBs, FUNCTIONs, and PROPERTYs. Fields appear as field-type completions and methods appear with call-signature snippets.
  - *Inheritance:* if your TYPE uses `EXTENDS`, the extension resolves the full inheritance chain and includes parent members in the completion list.

- **WITH block completion** — Inside a `WITH myForm ... END WITH` block, typing `.` automatically resolves `myForm`'s type and shows its members. No need to repeat the variable name.

- **CREATE block completion** — Inside a `CREATE myEdit AS PEDIT ... END CREATE` block, the extension offers the component's properties (with `= ` appended for quick assignment), methods, and events.

- **Type completion** — After `DIM x AS` or `CREATE x AS`, the extension lists all available data types, component types, and user-defined TYPE names.

- **Directive completion** — After typing `$`, all compiler directives are suggested with descriptions.

- **General completions** — At any other position, you get keyword completions, built-in function names (with snippet placeholders for arguments), and symbols you've defined in the current file (your own SUBs, FUNCTIONs, variables, constants, and CREATE'd components).

### Hover Documentation

The hover provider (`src/hoverProvider.js`) shows inline documentation when you hold the mouse or press the hover shortcut over any symbol:

- **Keywords** — Each keyword shows a brief description and typical syntax pattern. For example, hovering `FOR` shows: *"Counted loop. Syntax: FOR var = start TO end [STEP n] ... NEXT"*.
- **Component types** — Hovering a type like `PSTRINGGRID` displays all of its properties, methods, and events in categorised lists.
- **User-defined TYPEs** — Hovering a TYPE name (e.g., `Rect`) shows the full type definition: all fields with their types, all SUBs, FUNCTIONs, CONSTRUCTORs, and PROPERTYs with their signatures, and the parent type if `EXTENDS` is used.
- **User-defined TYPE members** — Hovering a member like `r.Left` (where `r` is a `Rect`) shows whether it's a field or method, its type or signature, and which TYPE it belongs to.
- **User-defined TYPE instances** — Hovering a variable like `r` (where `r` is a `DIM r AS Rect`) shows the variable's type and lists all available members.
- **Built-in functions** — Shows the function signature as a code block plus a description. For example, hovering `MID$` shows: `MID$(str, start [, length])` — *"Returns substring from position"*.
- **Data types** — Shows the type name and what Python type it maps to (e.g., `DOUBLE` → *"Double-precision float"*).
- **Directives** — Shows the directive's purpose (e.g., `$APPTYPE` → *"Set application type: GUI, CONSOLE, or CGI"*).
- **User variables** — If you hover a variable that was created with `CREATE`, it shows the variable name and its component type. If it's an instance of a user-defined TYPE, it shows the type name and its members.

### Signature Help

The signature provider (`src/signatureProvider.js`) activates when you type `(` after a built-in function name. It displays the full function signature in a tooltip and highlights the parameter you're currently typing. As you add commas to move to the next argument, the highlight advances accordingly.

This works for all 100+ built-in functions that accept parameters (string functions like `MID$`, `LEFT$`, `INSTR`; math functions like `ROUND`, `IIF`; GUI functions like `MESSAGEBOX`, `RGB`; etc.).

### Document Symbols & Outline

The symbol provider (`src/symbolProvider.js`) scans the current file and reports all significant declarations to VS Code. This powers:

- **The Outline panel** (sidebar) — Shows a tree of all SUBs, FUNCTIONs, TYPEs, CREATE blocks, CONSTs, and top-level DIM variables.
- **Breadcrumbs** (top of the editor) — Shows which SUB or FUNCTION the cursor is currently inside.
- **Go to Symbol** (`Cmd+Shift+O` / `Ctrl+Shift+O`) — Instantly jump to any declaration in the file.

Each symbol is tagged with an appropriate icon (function, struct, object, constant, variable) and shows its detail (e.g., a CREATE'd component shows its type like `PFORM`).

### Snippets (40+)

The extension includes over 40 code snippets (`snippets/rapidp.json`) that expand common patterns with a single Tab press. Snippet placeholders let you jump between fields with Tab.

| Prefix | What It Generates |
|--------|-------------------|
| `dim` | `DIM varName AS type` |
| `dima` | Array declaration with size |
| `const` | `CONST NAME = value` |
| `if` | `IF ... THEN ... END IF` |
| `ife` | `IF ... THEN ... ELSE ... END IF` |
| `ifeif` | `IF ... ELSEIF ... ELSE ... END IF` |
| `for` | `FOR i = 1 TO n ... NEXT` |
| `fors` | `FOR` with `STEP` |
| `while` | `WHILE ... WEND` |
| `dowhile` | `DO WHILE ... LOOP` |
| `dountil` | `DO ... LOOP UNTIL` |
| `dowhilepre` | `DO ... LOOP WHILE` (post-test) |
| `select` | `SELECT CASE` with branches |
| `sub` | `SUB Name() ... END SUB` |
| `func` | `FUNCTION Name() AS type ... END FUNCTION` |
| `type` | `TYPE Name ... END TYPE` |
| `typec` | TYPE with CONSTRUCTOR |
| `create-form` | Full `CREATE ... AS PFORM` block |
| `create-button` | Full `CREATE ... AS PBUTTON` block |
| `create-label` | Full `CREATE ... AS PLABEL` block |
| `create-edit` | Full `CREATE ... AS PEDIT` block |
| `create-panel` | Full `CREATE ... AS PPANEL` block |
| `create-timer` | Full `CREATE ... AS PTIMER` block |
| `create-listbox` | Full `CREATE ... AS PLISTBOX` block |
| `create-combobox` | Full `CREATE ... AS PCOMBOBOX` block |
| `create-grid` | Full `CREATE ... AS PSTRINGGRID` block |
| `create-richedit` | Full `CREATE ... AS PRICHEDIT` block |
| `create-canvas` | Full `CREATE ... AS PCANVAS` block |
| `create-codeeditor` | Full `CREATE ... AS PCODEEDITOR` block |
| `create-menu` | Full `CREATE ... AS PMAINMENU` block |
| `create-mysql` | Full `CREATE ... AS PMYSQL` block |
| `create-sqlite` | Full `CREATE ... AS PSQLITE` block |
| `create-socket` | Full `CREATE ... AS PSOCKET` block |
| `create-http` | Full `CREATE ... AS PHTTP` block |
| `with` | `WITH object ... END WITH` block |
| `import` | `IMPORT "module"` |
| `declare` | `DECLARE FUNCTION ... LIB ...` |
| `bind` | `BIND object.event TO handler` |
| `$include` | `$INCLUDE "file"` |
| `$define` | `$DEFINE SYMBOL value` |
| `$ifdef` | `$IFDEF ... $ENDIF` block |
| `$apptype` | `$APPTYPE GUI|CONSOLE|CGI` |
| `$typecheck` | `$TYPECHECK ON|OFF` |
| `rpcons` | Full console app skeleton with boilerplate |
| `rpgui` | Full GUI app skeleton (form, button, event handler) |
| `rpdb` | Full database app skeleton (SQLite connection, query, display) |

### Compile & Run Integration

The extension integrates directly with the RapidP compiler (`compile.py`) so you never need to switch to a terminal:

| Action | Shortcut | Command Palette |
|--------|----------|-----------------|
| Compile current file | `Ctrl+Shift+B` (`Cmd+Shift+B` on Mac) | *RapidP: Compile Current File* |
| Compile and run | `F5` | *RapidP: Compile and Run* |
| Build standalone executable | — | *RapidP: Build Standalone Executable* |

Additionally:
- A **play button** appears in the editor title bar when a `.rp` file is active.
- A **status bar item** (`▶ RapidP`) at the bottom of VS Code shows when a RapidP file is open. Click it to compile and run.
- The compiler path is **auto-detected** from your workspace (it searches for `compile.py` in the workspace root and parent directories). You can also set it manually in settings.

### Diagnostics

Every time you save a `.rp` file (and when you first open one), the extension performs lightweight validation and reports problems in the **Problems panel**:

- **Unclosed blocks** — Detects when a `IF` is missing its `END IF`, a `FOR` is missing its `NEXT`, a `SUB` is missing its `END SUB`, etc. The diagnostic appears on the opening line of the unclosed block, telling you exactly which closing keyword is missing.
- **Unterminated strings** — Detects lines where a string literal is opened with `"` but never closed.

These diagnostics appear as yellow/red squiggly underlines in the editor and in the Problems panel (`Ctrl+Shift+M` / `Cmd+Shift+M`).

---

## Extension Architecture

The extension is a pure JavaScript VS Code extension (no TypeScript compilation or bundler required). Here's how the source files are organised:

```
utilities/vscodeext/rapidp/
├── package.json                          # Extension manifest: commands, keybindings,
│                                         # settings, language & grammar registration
├── language-configuration.json           # Bracket matching, folding regions,
│                                         # auto-indentation rules, comment toggling
├── README.md                             # This file
├── syntaxes/
│   └── rapidp.tmLanguage.json            # TextMate grammar: regex-based tokenisation
│                                         # for syntax highlighting (15 pattern groups)
├── snippets/
│   └── rapidp.json                       # 40+ code snippets with Tab-stop placeholders
└── src/
    ├── extension.js                      # Main entry point: registers all providers,
    │                                     # commands, diagnostics, and status bar
    ├── languageData.js                   # Complete language database: 40+ component
    │                                     # types with props/methods/events, 100+
    │                                     # built-in functions, keywords, types, directives
    ├── completionProvider.js             # Context-aware autocomplete engine
    │                                     # (dot, WITH, CREATE, directive, type, general)
    │                                     # Supports both built-in and user-defined types
    ├── hoverProvider.js                  # Hover-to-see-docs for all language elements
    │                                     # including user-defined TYPE definitions
    ├── signatureProvider.js              # Function parameter hints on typing "("
    ├── symbolProvider.js                 # Document outline: SUBs, FUNCTIONs, TYPEs,
    │                                     # CREATE blocks, CONSTs, DIM variables
    └── typeParser.js                     # Shared parser for user-defined TYPE blocks:
                                          # extracts fields, SUBs, FUNCTIONs, PROPERTYs,
                                          # CONSTRUCTORs, and resolves EXTENDS inheritance
```

**How it works:**

1. When VS Code opens a `.rp` file, the extension activates (`onLanguage:rapidp` activation event).
2. The TextMate grammar handles all syntax highlighting passively — no JavaScript needed.
3. The four provider classes (`completionProvider`, `hoverProvider`, `signatureProvider`, `symbolProvider`) are registered with the VS Code language API and respond to editor events.
4. All providers share a single language database (`languageData.js`) containing the full RapidP component registry, built-in function catalogue, keyword list, type definitions, and directive definitions — all extracted directly from the RapidP compiler source code.
5. A shared `typeParser.js` module scans the document for `TYPE ... END TYPE` blocks on demand and extracts fields, methods, constructors, properties, and inheritance (`EXTENDS`). Both the completion and hover providers use this when a variable's type is not found in the built-in component registry.
6. The compile commands invoke `compile.py` in a VS Code terminal, passing the active file path and configured flags.
6. The diagnostic validator runs on save/open and pushes warnings to VS Code's Problems panel.

---

## Installation

### Method 1: Symlink (Development / Quick Setup)

The simplest way to install — create a symbolic link from VS Code's extensions folder to this directory:

```bash
# macOS / Linux
ln -s /path/to/RapidP/utilities/vscodeext/rapidp ~/.vscode/extensions/rapidp

# Example with the default RapidP project location:
ln -s ~/Programming/Python/RapidP/utilities/vscodeext/rapidp ~/.vscode/extensions/rapidp
```

Then reload VS Code:
- Press `Cmd+Shift+P` (Mac) or `Ctrl+Shift+P` (Windows/Linux)
- Type **Developer: Reload Window** and press Enter

Open any `.rp` file and the extension activates automatically.

### Method 2: Copy the Folder

If you prefer not to symlink, copy the entire extension folder:

```bash
cp -r /path/to/RapidP/utilities/vscodeext/rapidp ~/.vscode/extensions/rapidp
```

Reload VS Code as described above.

### Method 3: Build a VSIX Package

To create a distributable `.vsix` package that can be shared with others or installed on any machine:

```bash
cd /path/to/RapidP/utilities/vscodeext/rapidp

# Install the packaging tool (one-time)
npm install -g @vscode/vsce

# Build the VSIX
vsce package
```

This produces `rapidp-1.0.0.vsix`. Install it with:

```bash
code --install-extension rapidp-1.0.0.vsix
```

Or from within VS Code: `Extensions` sidebar → `...` menu → **Install from VSIX...** → select the file.

### Verifying the Installation

After installing and reloading:

1. Open a `.rp` file — you should see syntax colouring immediately.
2. Check the bottom-left status bar for the `▶ RapidP` indicator.
3. Try typing `DIM x AS ` — you should see type completions appear.
4. Open the Outline panel (`Cmd+Shift+O`) — SUBs and FUNCTIONs should appear.

---

## Configuration

All settings are under the `rapidp.*` namespace in VS Code Settings (`Cmd+,` / `Ctrl+,`):

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `rapidp.compilerPath` | `string` | `""` (auto-detect) | Absolute path to `compile.py`. When left empty, the extension searches the workspace root and up to two parent directories automatically. |
| `rapidp.pythonPath` | `string` | `python3` | The Python 3 interpreter used to run the compiler. Change this if your Python is installed under a different name (e.g., `python`, `/usr/local/bin/python3.11`). |
| `rapidp.runAfterCompile` | `boolean` | `true` | When enabled, the compiled Python output is executed immediately after a successful compile. Disable if you only want to transpile without running. |
| `rapidp.encoding` | `string` | `utf-8` | Source file encoding passed to the compiler. Choose `latin-1` if your `.rp` files use Latin-1 encoded characters. |

### Example `settings.json`

```json
{
    "rapidp.compilerPath": "/Users/you/Projects/RapidP/compile.py",
    "rapidp.pythonPath": "python3",
    "rapidp.runAfterCompile": true,
    "rapidp.encoding": "utf-8"
}
```

---

## Requirements

| Requirement | Version | Purpose |
|-------------|---------|---------|
| VS Code | 1.75 or later | Extension host API compatibility |
| Python | 3.x | Required to run `compile.py` (the RapidP transpiler) |
| RapidP compiler | — | The `compile.py` script and `compiler/` + `rp_runtime/` directories from the RapidP project |

No additional npm packages or native dependencies are required. The extension is pure JavaScript with zero runtime dependencies beyond the VS Code API.

---

## Supported Languages and File Types

| Language ID | File Extension | MIME Type |
|-------------|---------------|-----------|
| `rapidp` | `.rp` | `text/x-rapidp` |

---

## Known Limitations

- Diagnostics are lightweight (block matching and string checking only). Full semantic analysis (undefined variables, type mismatches) is handled by the compiler itself — use `Ctrl+Shift+B` to get full compiler diagnostics.
- The extension does not include a debugger. Run/debug is handled by compiling to Python and using Python's debugging tools.

---

## License

MIT

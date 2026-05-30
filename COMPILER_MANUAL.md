# RapidR Compiler & Language Manual

> **Purpose:** This is a comprehensive reference for AI assistants (and humans) working on the RapidR transpiler. It documents the Rust workspace, language syntax, compiler internals, runtime architecture, known limitations, and coding patterns.
> **Instructions for AI:** Always read this manual first when working on this repository. Keep this manual updated as features are added or changed.
>
> **Last Updated:** April 24, 2026

---

## Table of Contents

- [1. Project Overview](#1-project-overview)
- [1.1 RapidR Bootstrap Status](#11-rapidr-bootstrap-status)
- [2. Architecture & Pipeline](#2-architecture--pipeline)
- [3. Language Syntax Reference](#3-language-syntax-reference)
  - [3.1 Comments](#31-comments)
  - [3.2 Variables & Types](#32-variables--types)
  - [3.3 Constants](#33-constants)
  - [3.4 Arrays](#34-arrays)
  - [3.5 User-Defined Types (TYPE)](#35-user-defined-types-type)
  - [3.6 Control Flow](#36-control-flow)
  - [3.7 Subroutines & Functions](#37-subroutines--functions)
  - [3.8 CREATE Blocks (GUI)](#38-create-blocks-gui)
  - [3.9 WITH Blocks](#39-with-blocks)
  - [3.10 IMPORT Statement](#310-import-statement)
  - [3.11 DECLARE Statement (DLL/FFI)](#311-declare-statement-dllffi)
  - [3.12 BIND Statement](#312-bind-statement)
  - [3.13 PRINT Statement](#313-print-statement)
  - [3.14 Operators](#314-operators)
  - [3.15 String Suffixes](#315-string-suffixes)
- [4. Preprocessor Directives](#4-preprocessor-directives)
- [5. Compiler Internals](#5-compiler-internals)
  - [5.1 Lexer (rapidr-lexer)](#51-lexer-rapidr-lexer)
  - [5.2 Parser (rapidr-parser)](#52-parser-rapidr-parser)
  - [5.3 AST (rapidr-ast)](#53-ast-rapidr-ast)
  - [5.4 Code Generator (rapidr-codegen-rust)](#54-code-generator-rapidr-codegen-rust)
  - [5.5 Preprocessor (rapidr-preprocessor)](#55-preprocessor-rapidr-preprocessor)
  - [5.6 Diagnostics (rapidr-diagnostics)](#56-diagnostics-rapidr-diagnostics)
  - [5.7 Symbol Table](#57-symbol-table)
  - [5.8 Component Registry](#58-component-registry)
- [6. Runtime Library](#6-runtime-library)
  - [6.1 Builtins (builtins.rs)](#61-builtins-builtinsrs)
  - [6.2 GUI (gui.rs)](#62-gui-guirs)
  - [6.3 Database (database.rs)](#63-database-databasers)
  - [6.4 Network (network.rs)](#64-network-networkrs)
  - [6.5 Data Science (datascience.rs)](#65-data-science-datasciencers)
- [6b. Web Runtime (rapidr-runtime-web)](#6b-web-runtime-rapidr-runtime-web)
- [6c. Bytecode Interpreter (rapidrintr)](#6c-bytecode-interpreter-rapidrintr)
- [6d. Self-Hosted Web IDE (web-ide/)](#6d-self-hosted-web-ide-web-ide)
- [7. Code Generation Patterns](#7-code-generation-patterns)
- [8. Known Limitations & Gotchas](#8-known-limitations--gotchas)
- [9. How to Add New Features](#9-how-to-add-new-features)
- [10. Test Suite](#10-test-suite)

---

## 1. Project Overview

**RapidR** is a BASIC-to-Rust transpiler that reads `.rr` source files (BASIC-like syntax inspired by RapidQ) and produces standalone Rust projects that compile to native executables. The project directory structure:

```
crates/                 — RapidR Rust workspace crates (9 crates)
  rapidr-cli/           — CLI entry point (version, lex, parse, codegen, shortcut build)
  rapidr-diagnostics/   — TextSpan, SourceLocation, Diagnostic types
  rapidr-ast/           — Shared AST data structures
  rapidr-lexer/         — Tokenization (keywords, literals, operators, directives)
  rapidr-parser/        — Recursive-descent parser → AST
  rapidr-preprocessor/  — $INCLUDE, $DEFINE, $IFDEF, $MACRO, $THEME
  rapidr-codegen-rust/  — AST → Rust source code generator
  rapidr-runtime-core/  — Native runtime (GUI, builtins, database, networking, data science)
  rapidr-runtime-web/   — Web/WASM runtime (DOM GUI, web builtins, in-memory DB, data science)
examples/               — Demo and test .rr programs
utilities/vscodeext/    — VS Code extension
```

### 1.1 RapidR Bootstrap Status

The Rust migration has reached **functional transpiler status** with a complete pipeline from `.rr` source to native executables or WebAssembly. The Cargo workspace contains 9 crates:

| Crate | Lines | Description |
|-------|-------|-------------|
| `rapidr-cli` | — | CLI with `version`, `preprocess`, `lex`, `parse`, `codegen` commands |
| `rapidr-diagnostics` | — | `TextSpan`, `SourceLocation`, and `Diagnostic` types |
| `rapidr-ast` | — | Shared AST data structures |
| `rapidr-preprocessor` | — | Directives: `$DEFINE`, `$UNDEF`, `$IFDEF`, `$IFNDEF`, `$ELSE`, `$ENDIF`, `$MACRO`, `$INCLUDE`, `$THEME` |
| `rapidr-lexer` | — | Lexer covering keywords, literals, directives, operators, suffixes, line continuations |
| `rapidr-parser` | — | Recursive-descent parser producing typed AST |
| `rapidr-codegen-rust` | ~2,100 | **Rust code generator** — walks AST, emits Rust source targeting `rapidr-runtime-core` or `rapidr-runtime-web` |
| `rapidr-runtime-core` | ~5,700 | **Native runtime** — FLTK GUI (~2,200 lines), builtins, database (MySQL/SQLite), networking, file I/O |
| `rapidr-runtime-web` | ~5,600 | **Web runtime** — DOM/Canvas GUI, web builtins, in-memory SQLite, data science, wasm-bindgen interop |

**Key architecture decisions:**
- Generated code uses `thread_local!` storage for module-level variables (`gv()`/`gs()` scalar accessors, `ga_get()`/`ga_set()` array accessors), correctly sharing state across SUBs/FUNCTIONs
- GUI components use FLTK via the `fltk` crate with `fltk-theme` for theming
- Component properties/methods are dispatched through a centralized `rp_comp_get`/`rp_comp_set`/`rp_comp_method` API backed by thread-local `GUI_COMPONENTS` storage
- UDT variables remain as native Rust structs (not stored in the global variable HashMap)

**Current validation:**
- All Rust unit tests pass across all crates
- All 29 example programs generate, compile, and run
- The self-hosted IDE (`examples/ide.rr`) compiles to a native FLTK application with working properties, events, code view, and design surface, still WIP but demonstrates the full pipeline.

```bash
cargo test                  # Run all tests
cargo run -- codegen examples/ide.rr /tmp/ide_rust  # Generate IDE
cd /tmp/ide_rust && cargo build && ./target/debug/ide  # Build and run
```

### CLI Usage

```bash
# Shortcut syntax — builds and places binary alongside source
rapidr --release examples/hello_world.rr
rapidr --debug examples/hello_world.rr

# Web compilation — builds WASM and generates HTML/JS output
rapidr --web examples/hello_web.rr

# Full subcommand syntax
rapidr codegen <file.rr> <outdir> [--release|--debug]
rapidr codegen --web <file.rr>
rapidr lex <file.rr>
rapidr parse <file.rr>
rapidr preprocess <file.rr>
rapidr version
```

| Command | Description |
|---------|-------------|
| `codegen <file.rr> <outdir>` | Generate a Rust project from a `.rr` file |
| `codegen --web <file.rr>` | Generate a WASM web project, compile to wasm32, run wasm-bindgen, produce HTML/JS/WASM output |
| `lex <file.rr>` | Dump token stream |
| `parse <file.rr>` | Dump AST |
| `preprocess <file.rr>` | Dump preprocessed source |
| `version` | Print version |

Optional flags for `codegen`:

| Flag | Description |
|------|-------------|
| `--release` | Build in release mode (optimized) |
| `--debug` | Build in debug mode (default) |
| `--web` | Compile to WebAssembly targeting `rapidr-runtime-web` instead of `rapidr-runtime-core` |

---

## 2. Architecture & Pipeline

The compilation pipeline:

```
Source (.rr)
    ↓
Preprocessor (rapidr-preprocessor) — $INCLUDE, $DEFINE, $IFDEF, $MACRO, $THEME expansion
    ↓
Lexer (rapidr-lexer)               — Tokenization into Token stream
    ↓
Parser (rapidr-parser)             — Recursive-descent → AST (Program)
    ↓
CodeGenerator (rapidr-codegen-rust) — AST traversal → Rust source code
    ↓
Output (Rust project)               — Cargo.toml + src/main.rs targeting rapidr-runtime-core
    ↓
cargo build                         — Compiles to native executable
```

**Key design decisions:**
- The parser is a **single-pass recursive-descent** parser — no separate semantic analysis pass.
- The code generator does **three pre-passes** before generating code:
  1. Collect UDT type names
  2. Collect global variables, arrays, constants, and CREATE components
  3. Collect SUB/FUNCTION signatures, DECLARE, IMPORT modules
- Then processes directives ($TYPECHECK, $APPTYPE, $THEME, etc.)
- Then generates Rust code by visiting AST nodes.

---

## 3. Language Syntax Reference

### 3.1 Comments

```basic
' This is a comment (single quote)
REM This is also a comment
```

Comments are stripped by the lexer. Everything after `'` or `REM` on a line is ignored.

### 3.2 Variables & Types

```basic
DIM x AS INTEGER
DIM name AS STRING
DIM value AS DOUBLE
DIM flag AS LONG
DIM anything AS VARIANT      ' Default type when unspecified
```

**Supported types:**
| Type | Rust Equivalent | Default Value |
|------|-----------------|---------------|
| `INTEGER` | `i64` | `0` |
| `LONG` | `i64` | `0` |
| `INT64` | `i64` | `0` |
| `BYTE` | `i64` | `0` |
| `WORD` | `i64` | `0` |
| `DWORD` | `i64` | `0` |
| `SINGLE` | `f64` | `0.0` |
| `DOUBLE` | `f64` | `0.0` |
| `CURRENCY` | `f64` | `0.0` |
| `STRING` | `String` | `""` |
| `VARIANT` | `Value` | `v_null()` |
| `ROBJECT` | `Value` | `v_null()` |
| Any P-component | component instance | `rp_comp_create()` |
| Any UDT name | struct instance | constructor call |

**Multiple declarations:**
```basic
DIM a, b, c AS INTEGER     ' All three are INTEGER
```

**Implicit declaration:** When `$TYPECHECK OFF` (default), assigning to an undeclared variable auto-declares it as `DOUBLE` (or whatever `$OPTION DIM` specifies).

**Case insensitivity:** Identifiers are case-insensitive. The codegen lowercases all identifiers (e.g., `MyVar` → `myvar`).

**Suffix conventions (legacy BASIC):** The lexer accepts `$`, `%`, `#`, `&`, `!` suffixes on identifiers (e.g., `name$`, `count%`). These are stripped during code generation.

### 3.3 Constants

```basic
CONST PI = 3.14159
CONST MAX_SIZE AS LONG = 100    ' Optional type annotation (ignored)
CONST GREETING = "Hello"
```

Constants are compiled as regular variable assignments.

### 3.4 Arrays

```basic
DIM values(100) AS DOUBLE       ' Array of 101 elements (0-100)
DIM grid(10, 20) AS INTEGER     ' 2D array (currently stored as nested list)
DIM names(1 TO 50) AS STRING    ' 1-based range (allocates 51 elements, 0 unused)
```

**Important:** In RapidR, `DIM A(10)` creates 11 elements (indices 0-10), matching classic BASIC semantics.

**Array access uses parentheses** (not square brackets):
```basic
values(5) = 42.0
x = values(i)
```

**The array/function ambiguity:** Both array access and function calls use `()` — e.g., `foo(5)` could be either. The parser always creates `FunctionCallNode` at parse time. The codegen resolves this:
1. If the name is in `self.arrays` set → emit `name[idx]`
2. If the name is in the symbol table as `variable`/`array`/`component` → emit `name[idx]`
3. Otherwise → emit `name(idx)` (function call)

### 3.5 User-Defined Types (TYPE)

```basic
TYPE PersonType
    Name AS STRING
    Age AS INTEGER
    Scores(10) AS DOUBLE
END TYPE

TYPE Employee EXTENDS PersonType
    Department AS STRING
    
    CONSTRUCTOR
        .Department = "Unknown"
    END CONSTRUCTOR
    
    SUB Display()
        PRINT .Name + " - " + .Department
    END SUB
END TYPE

DIM p AS PersonType
p.Name = "Alice"
p.Age = 30
p.Scores(1) = 95.5
```

**Generated as Rust structs with `Default` trait implementation.**

Supports:
- Fields with optional array dimensions
- `EXTENDS` for inheritance
- `CONSTRUCTOR` block (mapped to `__post_init__`)
- Embedded `SUB`/`FUNCTION` methods (gets `self` parameter injected)
- `PRIVATE`/`PUBLIC` visibility markers (parsed but ignored in codegen)
- `PROPERTY SET` attribute (parsed but not fully implemented)

### 3.6 Control Flow

#### IF...THEN...END IF (Block)
```basic
IF x > 10 THEN
    PRINT "Large"
ELSEIF x > 5 THEN
    PRINT "Medium"
ELSE
    PRINT "Small"
END IF
```

#### IF...THEN (Single-line)
```basic
IF x > 10 THEN PRINT "Large"
IF x > 10 THEN PRINT "A" : PRINT "B" ELSE PRINT "C"
```
Single-line IF handles `:` as additional THEN-branch statements until ELSE or newline.

#### FOR...NEXT
```basic
FOR i = 1 TO 10
    PRINT STR$(i)
NEXT i           ' Variable name after NEXT is optional

FOR j = 10 TO 0 STEP -1
    PRINT STR$(j)
NEXT j
```

**Generated as `while` loop** in Rust:
```rust
gs("i", v_int(1));
while (gv("i").rp_le(&v_int(10))).to_bool() {
    rp_print(&[rp_str_func(&gv("i"))], true);
    gs("i", &gv("i") + &v_int(1));
}
```
This correctly handles both positive and negative STEP values, including floating-point steps.

#### WHILE...WEND
```basic
WHILE x < 100
    x = x * 2
WEND
```

#### DO...LOOP
```basic
' Pre-condition forms:
DO WHILE x < 100
    x = x + 1
LOOP

DO UNTIL x >= 50
    x = x + 1
LOOP

' Post-condition forms:
DO
    x = x + 1
LOOP WHILE x < 100

DO
    x = x + 1
LOOP UNTIL x >= 50
```

#### SELECT CASE
```basic
SELECT CASE grade
    CASE "A"
        PRINT "Excellent"
    CASE "B", "C"
        PRINT "Good"
    CASE ELSE
        PRINT "Try harder"
END SELECT
```
**Generated as if/elif/else chain** with a temporary variable `_select_val_{line}`.

#### EXIT Statement
```basic
EXIT FOR       ' → break
EXIT WHILE     ' → break
EXIT DO        ' → break
EXIT SUB       ' → return
EXIT FUNCTION  ' → return <function_name>
```

### 3.7 Subroutines & Functions

#### SUB (no return value)
```basic
SUB Greet(name AS STRING)
    PRINT "Hello, " + name
END SUB

' Calling:
CALL Greet("World")
Greet "World"           ' CALL keyword is optional
Greet("World")          ' Parentheses also work
```

#### FUNCTION (returns a value)
```basic
FUNCTION Add(a AS INTEGER, b AS INTEGER) AS INTEGER
    Add = a + b          ' Return by assigning to function name
END FUNCTION

' Or with RETURN:
FUNCTION Add2(a AS INTEGER, b AS INTEGER) AS INTEGER
    RETURN a + b
END FUNCTION

DIM result AS INTEGER
result = Add(3, 4)
```

**Function return mechanism:** The codegen creates a local variable with the function's name, initialized to `None`. Assigning to the function name sets this variable. A `return <name>` is appended at the end.

**Parameter passing:** `BYVAL` and `BYREF` keywords are accepted by the parser. In the Rust codegen, all parameters are passed by value (clone semantics).

**Global variable scoping:** Global variables are accessed via `gv()`/`gs()` thread-local storage. All SUB/FUNCTION bodies automatically access globals through these functions without explicit declarations.

### 3.8 CREATE Blocks (GUI)

```basic
CREATE Form1 AS RForm
    Caption = "My App"
    Width = 640
    Height = 480
    
    CREATE Panel1 AS RPanel
        Align = 5
        
        CREATE Button1 AS RButton
            Caption = "OK"
            Left = 10
            Top = 10
            OnClick = HandleOK
        END CREATE
    END CREATE
END CREATE
```

**Semantics:**
- `CREATE X AS RType` generates `rp_comp_create("x", "RTYPE")` plus `rp_comp_set` for the parent
- Property assignments inside CREATE become `rp_comp_set("x", "prop", value)`
- Method calls inside CREATE become `rp_comp_method("x", "method", &[args])`
- Nested CREATEs pass the parent automatically
- Builtin function calls inside CREATE blocks are NOT prefixed (they're detected via a known-builtins list)
- The `create_stack` tracks nesting for implicit parent assignment

**Q-prefix backward compatibility:** Types prefixed with `Q` (from RapidQ) are automatically normalized to `R` prefix in the codegen.

### 3.9 WITH Blocks

```basic
DIM form AS RForm

WITH form
    .Caption = "My Form"
    .Width = 800
    .Height = 600
END WITH
```

The parser maintains a `with_stack`. When a `.` is encountered at the start of an expression or statement, it's resolved to `with_stack[-1].member`.

### 3.10 IMPORT Statement

```basic
IMPORT "math" AS math
```

Generated as a comment in Rust. Data science functionality is provided via `RNum`, `RPlot`, and `RDataFrame` component types.

### 3.11 DECLARE Statement (DLL/FFI)

```basic
DECLARE SUB Sleep LIB "kernel32" ALIAS "Sleep" (ms AS LONG)
DECLARE FUNCTION GetTickCount LIB "kernel32" ALIAS "GetTickCount" () AS LONG
```

Translated to FFI calls via `libloading`. The shared library is loaded at runtime and functions are called through the `ffi.rs` module.

### 3.12 BIND Statement

```basic
BIND Button1.OnClick TO HandleClick
```

Generated as `button1.onclick = handleclick`.

### 3.13 PRINT Statement

```basic
PRINT "Hello"                    ' With newline
PRINT "A", "B", "C"             ' Comma-separated (space-separated in output)
PRINT "No newline";             ' Semicolon suppresses newline
PRINT                           ' Empty line
PRINT #1, "To file"             ' File I/O (handled by builtins)
```

Generated as `rp_print(...)` calls in the runtime.

### 3.14 Operators

| Operator | BASIC | Rust Equivalent |
|----------|-------|-----------------|
| Addition | `+` | `+` |
| Subtraction | `-` | `-` |
| Multiplication | `*` | `*` |
| Division | `/` | `/` |
| Integer Division | `\` | `/ (int)` |
| Modulo | `MOD` | `%` |
| Exponentiation | `^` | `.powf()` |
| String Concatenation | `&` or `+` | `format!()` |
| Equal | `=` | `==` |
| Not Equal | `<>` | `!=` |
| Less Than | `<` | `<` |
| Greater Than | `>` | `>` |
| Less/Equal | `<=` | `<=` |
| Greater/Equal | `>=` | `>=` |
| Logical AND | `AND` | `&&` |
| Logical OR | `OR` | `\|\|` |
| Logical NOT | `NOT` | `!` |
| Logical XOR | `XOR` | `^` |

**Operator precedence (highest to lowest):**
1. Unary (`NOT`, `-`, `+`)
2. Exponentiation (`^`)
3. Multiplication/Division (`*`, `/`, `\`, `MOD`)
4. Addition/Subtraction (`+`, `-`, `&`)
5. Comparison (`<`, `>`, `<=`, `>=`)
6. Equality (`=`, `<>`)
7. Logical AND
8. Logical OR, XOR

### 3.15 String Suffixes

BASIC-style identifier suffixes are accepted by the lexer:
- `$` — String (e.g., `LEFT$`, `MID$`, `name$`)
- `%` — Integer
- `#` — Double
- `&` — Long
- `!` — Single

These are **stripped during code generation** — they don't affect typing.

---

## 4. Preprocessor Directives

Handled in `compiler/preprocessor.py` before lexing.

| Directive | Description | Behavior |
|-----------|-------------|----------|
| `$INCLUDE "file"` | Include external source file | Recursively preprocesses and inlines; detects circular includes |
| `$DEFINE SYMBOL [value]` | Define text substitution | Whole-word replacement in non-string segments; default value = `"1"` |
| `$UNDEF SYMBOL` | Remove a define | |
| `$IFDEF SYMBOL` | Conditional: skip if not defined | Supports nesting |
| `$IFNDEF SYMBOL` | Conditional: skip if defined | |
| `$ELSE` | Toggle conditional branch | |
| `$ENDIF` | End conditional block | |
| `$MACRO NAME[(params)] = body` | Define parameterized macro | Expanded before $DEFINE substitution |
| `$APPTYPE GUI\|CONSOLE\|CGI` | Set application type | Passed through to codegen as DirectiveNode |
| `$OPTIMIZE ON\|OFF` | Optimization hint | Pass-through (no effect) |
| `$ESCAPECHARS ON\|OFF` | Escape character mode | Pass-through (no effect) |
| `$TYPECHECK ON\|OFF` | Enable/disable strict type checking | Enables undeclared variable/function errors |
| `$OPTION EXPLICIT` | Same as `$TYPECHECK ON` | |
| `$OPTION DIM <TYPE>` | Default DIM type | Changes default from `DOUBLE` |
| `$THEME <name>` | Set FLTK theme (Rust only) | Applied at program start; available themes: `Classic`, `Aero`, `Metro`, `AquaClassic`, `Greybird`, `Blue`, `Dark`, `HighContrast`, `AUTO` |

**Line preservation:** The preprocessor replaces consumed directive lines with empty strings to preserve line numbers for error reporting.

**$DEFINE substitution rules:**
- Only replaces whole words (uses `\b` word boundaries)
- Skips content inside quoted strings (splits by `"`, only substitutes in even-indexed segments)
- Sorted by length (longest first) to avoid partial replacements
- Inline comments in `$DEFINE` lines are stripped via `split("'")`

---

## 5. Compiler Internals

### 5.1 Lexer (`compiler/lexer.py`)

**307 lines.** Tokenizes source into `Token(type, value, line, column)` tuples.

**Token types (enum `TokenType`):**
- **Keywords (40+):** `DIM`, `AS`, `IF`, `THEN`, `ELSE`, `ELSEIF`, `END`, `FOR`, `TO`, `STEP`, `NEXT`, `WHILE`, `WEND`, `DO`, `LOOP`, `UNTIL`, `SELECT`, `CASE`, `SUB`, `FUNCTION`, `CALL`, `RETURN`, `EXIT`, `PRINT`, `INPUT`, `GOTO`, `GOSUB`, `IMPORT`, `CREATE`, `CONST`, `TYPE`, `DECLARE`, `LIB`, `ALIAS`, `WITH`, `EXTENDS`, `PROPERTY`, `SET`, `BYVAL`, `BYREF`, `BIND`, `CONSTRUCTOR`, `AND`, `OR`, `NOT`, `XOR`, `MOD`, `DEFSTR`, `DEFINT`, etc.
- **Type keywords:** `INTEGER`, `STRING`, `DOUBLE`, `SINGLE`, `BYTE`, `WORD`, `DWORD`, `LONG`, `INT64`, `CURRENCY`, `POBJECT`
- **Operators:** `PLUS`, `MINUS`, `STAR`, `SLASH`, `BACKSLASH`, `CARET`, `AMPERSAND`, `EQ`, `NEQ`, `LT`, `LTE`, `GT`, `GTE`
- **Symbols:** `LPAREN`, `RPAREN`, `COMMA`, `COLON`, `SEMI`, `DOT`
- **Literals:** `NUMBER`, `STRING_LIT`, `IDENTIFIER`
- **Special:** `NEWLINE`, `EOF`, `DIRECTIVE`

**Tokenization method:** Single compiled regex with named groups, applied via `finditer()`. Rules are ordered by priority (comments first, then directives, numbers, strings, identifiers, operators).

**Number formats:**
- Decimal: `123`, `3.14`, `1.5e10`
- Hex: `&H1F` or `&hFF` → converted to `0x1f`
- Octal: `&O77` → converted to `0o77`
- Binary: `&B1010` → converted to `0b1010`

**Line continuation:** `_` at end of line (before newline) is swallowed as whitespace.

**CRITICAL LIMITATION:** The lexer does NOT support `[`, `]`, `{`, or `}` characters. Workarounds: use component methods (RNum.arange, RDataFrame.loadfromcsv) or RUSTSTART/RUSTEND blocks for raw Rust code.

### 5.2 Parser (`compiler/parser.py`)

**847 lines.** Recursive-descent parser producing AST from token stream.

**Key methods:**
- `parse()` → `ProgramNode` (top-level)
- `parse_statement()` — dispatches to specific statement parsers based on current token
- `parse_expression()` → starts precedence climbing
- `parse_assignment_or_call()` — handles the ambiguous `IDENTIFIER [.member]* [= expr | args]` pattern

**Expression precedence (method call chain):**
```
parse_expression → parse_logical_or → parse_logical_and → parse_equality
    → parse_comparison → parse_term → parse_factor → parse_power
    → parse_unary → parse_primary
```

**Statement terminators:** `NEWLINE` or `COLON` (`:` allows multiple statements per line).

**Block parsing:** `parse_block(end_tokens)` — consumes statements until one of the `end_tokens` is found at the current position. Used by IF, FOR, WHILE, DO, SELECT, SUB, FUNCTION, TYPE, WITH, CREATE.

**The assignment-or-call ambiguity:** When the parser sees an `IDENTIFIER`:
1. If followed by `(` → parse as `ArrayAccessNode` (statement level)
2. If followed by `.` → chain into `MemberAccessNode`
3. If followed by `=` → it's an `AssignmentNode`
4. Otherwise → it's a `CallStatementNode` with space-separated arguments

**The array/function ambiguity in expressions:** In `parse_primary()`, `IDENTIFIER(args)` is always parsed as `FunctionCallNode`. The codegen resolves whether it's actually array indexing.

### 5.3 AST (`rapidr-ast`)

Located in `crates/rapidr-ast/src/lib.rs`. Defines all AST node types as Rust enums and structs.

**Expression nodes:**
- `Identifier(name)` — variable or function name
- `Literal(value, type_name)` — string, number literal
- `BinaryOp(left, op, right)` — binary operation
- `UnaryOp(op, operand)` — unary operation
- `ArrayAccess(array, index)` — explicit array access
- `MemberAccess(obj, member)` — dot access
- `FunctionCall(name, args)` — function call OR array access (ambiguous)
- `MethodCall(obj, method, args)` — method call
- `RustBlock(code)` — raw Rust code (RUSTSTART/RUSTEND)

**Statement nodes:**
- `DimStatement`, `AssignmentStatement`, `IfStatement`, `ForStatement`, `WhileStatement`
- `DoLoopStatement`, `SelectCaseStatement`, `PrintStatement`
- `SubroutineDef`, `FunctionDef`, `CallStatement`, `MethodCallStatement`
- `ImportStatement`, `WithStatement`, `CreateStatement`
- `ReturnStatement`, `ExitStatement`, `DirectiveStatement`
- `ConstStatement`, `DeclareStatement`, `TypeStatement`, `BindStatement`

### 5.4 Code Generator (`rapidr-codegen-rust`)

Located in `crates/rapidr-codegen-rust/src/lib.rs` (~2,200 lines). Walks AST and emits Rust source code targeting `rapidr-runtime-core`.

**Key state:**
- `output` — String buffer for generated code
- `indent` — current Rust indentation depth
- `global_vars`, `arrays`, `consts` — collected from pre-passes
- `components` — set of CREATE'd component names
- `udts` — user-defined type names
- `sub_signatures`, `func_signatures` — SUB/FUNCTION parameter info
- `create_stack` — stack of current CREATE block names (for implicit property prefixing)

**Three pre-passes before code generation:**
1. Collect UDT type names
2. Collect global variables, arrays, constants, and CREATE components
3. Collect SUB/FUNCTION signatures, DECLARE, IMPORT modules

**Generated Rust structure:**
```rust
use rapidr_runtime_core::*;
// + thread_local globals, helper functions
fn main() {
    // Global initializations (gs/ga_init/rp_comp_create)
    // Top-level statements
    // gui_showmodal() if GUI app
}
// SUB/FUNCTION definitions as standalone fn's
```

### 5.5 Preprocessor (`rapidr-preprocessor`)

Located in `crates/rapidr-preprocessor/src/lib.rs`. Pure text-based preprocessing before lexing.

Key features:
- Recursive `$INCLUDE` with circular-include detection
- Nested `$IFDEF`/`$IFNDEF` with skip stack
- `$MACRO` with optional parameters
- `$THEME` for FLTK theming
- Line-number preservation

### 5.6 Diagnostics (`rapidr-diagnostics`)

Located in `crates/rapidr-diagnostics/src/lib.rs`. Defines `TextSpan`, `SourceLocation`, and `Diagnostic` types used throughout the compiler for error reporting with line/column information.

### 5.7 Symbol Table

The codegen maintains symbol information collected during pre-passes. Global variables, arrays, constants, component names, and SUB/FUNCTION signatures are all tracked for correct code emission.

### 5.8 Component Registry

The component registry is distributed across:
- `crates/rapidr-runtime-core/src/gui.rs` — widget creation (`gui_create_widget`)
- `crates/rapidr-runtime-core/src/object.rs` — property get/set/method dispatch
- `crates/rapidr-codegen-rust/src/lib.rs` — `is_component_type_name()` and `is_component_method_name()`

Covers 52+ component types: `RFORM`, `RBUTTON`, `RLABEL`, `REDIT`, `RCANVAS`, `RPANEL`, `RTIMER`, `RSTRINGGRID`, `RIMAGE`, `RCODEEDITOR`, `RMYSQL`, `RSQLITE`, `RSOCKET`, `RSERVERSOCKET`, `RHTTP`, `RNUM`, `RPLOT`, `RDATAFRAME`, `RJSON`, `RCOOLBTN`, `ROVALBTN`, `RDESIGNSURFACE`, `RTREEVIEW`, `RLISTVIEW`, `RSPLITTER`, `RTRACKBAR`, `RSCROLLBOX`, `RPOPUPMENU`, `RINI`, `RMEMORYSTREAM`, `RSTRINGLIST`, `RPRINTER`, `RCOLORDIALOG`, `RFONTDIALOG`, `RSTATUSBAR`, `RLINE`, `RICON`, `RIMAGELIST`, etc.

---

## 6. Runtime Library

The Rust runtime is in `crates/rapidr-runtime-core/src/` with modules:

| Module | Lines | Description |
|--------|-------|-------------|
| `value.rs` | ~300 | `Value` enum (Int, Dbl, Str, Null) with arithmetic and comparison operators |
| `builtins.rs` | ~450 | 100+ built-in BASIC functions (string, math, I/O, system) |
| `gui.rs` | ~3,600 | FLTK-based GUI — 49+ component types, event system, design surface |
| `object.rs` | ~1,200 | Component property get/set/method dispatch via `rp_comp_*` API |
| `database.rs` | ~350 | MySQL (`mysql` crate) and SQLite (`rusqlite` crate) components |
| `network.rs` | ~400 | TCP socket, server socket, HTTP client components |
| `datascience.rs` | ~700 | `RNum` (ndarray), `RPlot` (plotters + image), `RDataFrame` (polars) |
| `file_io.rs` | ~200 | RFileStream, RIni, RMemoryStream, RStringList implementations |
| `ffi.rs` | ~200 | DECLARE/DLL foreign function interface via `libloading` |

### 6.1 Builtins (`builtins.rs`)

Implements 100+ BASIC functions as Rust functions operating on the `Value` type:

- **String functions:** `rp_chr`, `rp_asc`, `rp_left`, `rp_right`, `rp_mid`, `rp_len`, `rp_instr`, `rp_ucase`, `rp_lcase`, `rp_trim`, `rp_replace`, etc.
- **Math functions:** `rp_abs`, `rp_sin`, `rp_cos`, `rp_sqr`, `rp_rnd`, `rp_round`, `rp_ceil`, `rp_floor`, etc.
- **I/O functions:** `rp_print`, `rp_dir`, `rp_fileexists`, `rp_direxists`, `rp_mkdir`, `rp_rmdir`, `rp_kill`, `rp_rename`
- **System functions:** `rp_shell`, `rp_sleep`, `rp_timer`, `rp_date`, `rp_time`, `rp_environ`, `rp_command`
- **GUI functions:** `rp_showmessage`, `rp_messagebox`, `rp_rgb`

### 6.2 GUI (`gui.rs`)

The largest runtime module (~3,600 lines). Implements 49+ FLTK-based GUI components.

**Component lifecycle:**
1. `rp_comp_create(name, type)` — registers component in `GUI_COMPONENTS` thread-local
2. `gui_create_widget(name, type)` — creates FLTK widget, stores in `GUI_WIDGETS`
3. `rp_comp_set(name, prop, value)` — sets properties (before or after widget creation)
4. `gui_showmodal(form_name)` — starts FLTK event loop

**Key component types:**
- `RFORM` — FLTK `Window` with menu bar, status bar, timer support
- `RBUTTON` — Push button with hover/press color feedback
- `RLABEL`, `REDIT`, `RRICHEDIT` — Text display/input
- `RCANVAS` — Drawing surface with pset/line/circle/fillrect/textout methods
- `RSTRINGGRID` — Editable grid with column/row management
- `RCODEEDITOR` — Syntax-highlighted text editor with line numbers
- `RCOOLBTN` — Flat/toggle toolbar button with optional multi-state BMP images, GroupIndex for radio-group behavior
- `ROVALBTN` — Oval/round button with customizable Color, ColorHighlight, ColorShadow
- `RDESIGNSURFACE` — Visual form designer (used by the IDE)
- `RFONTDIALOG` — Font picker with preview
- `RCOLORDIALOG` — System color picker
- `RJSON` — JSON parsing, generation, dot-path get/set, file I/O (cross-platform)

### 6.3 Database (`database.rs`)

- **RMYSQL** — MySQL client via `mysql` crate. Connect, query, fetch rows/fields, iterate databases/tables.
- **RSQLITE** — SQLite via `rusqlite` crate. Same interface as RMYSQL.

### 6.4 Network (`network.rs`)

- **RSOCKET** — TCP client with connect/write/read, event callbacks
- **RSERVERSOCKET** — Threaded TCP server with per-client management, broadcast
- **RHTTP** — HTTP GET/POST client via `ureq` crate

### 6.5 Data Science (`datascience.rs`)

- **RNUM** — Numeric arrays via `ndarray` crate. Methods: `zeros`, `ones`, `arange`, `linspace`, `reshape`, `sum`, `mean`, `std`, `min`, `max`, `dot`, `transpose`, `sort`, `savetofile`, `loadfromfile`.
- **RPLOT** — Chart generation via `plotters` crate with in-memory PNG encoding via `image` crate. Methods: `plot`, `scatter`, `bar`, `hist`, `pie`, `legend`, `clear`, `savefig`. Plot rendering uses `BitMapBackend::with_buffer()` to render into an RGB pixel buffer, then encodes to PNG bytes in memory — no temporary files are created. `RImage.loadfromplot` loads the PNG bytes directly via `PngImage::from_data()`.
- **RDATAFRAME** — DataFrames via `polars` crate. Methods: `loadfromcsv`, `savetocsv`, `loadfromjson`, `savetojson`, `head`, `tail`, `describe`, `sort`, `filter`, `groupby`, `addcolumn`, `deletecolumn`, `cell`, `setcell`, `query`, `tostring`, `tolist`.

### 6.6 JSON (`object.rs`)

- **RJSON** — JSON parsing and generation via `serde_json` crate. Methods: `parse`, `stringify`, `prettify`, `get` (dot-path with array index support), `set` (dot-path with auto-create), `has`, `remove`, `count`, `keys`, `loadfile`, `savefile`, `clear`. Cross-platform: desktop uses `serde_json`, web uses `js_sys::JSON` and `Reflect` API.

---

## 6b. Web Runtime (rapidr-runtime-web)

The `rapidr-runtime-web` crate (`crates/rapidr-runtime-web/`) provides a browser-based runtime that mirrors the native `rapidr-runtime-core` API using web-sys, js-sys, and wasm-bindgen. When the codegen runs with `--web`, it targets this crate instead of `rapidr-runtime-core`.

### Web Runtime Modules

| Module | File | Purpose |
|--------|------|---------|
| `gui_web` | `gui_web.rs` (~2,300 lines) | HTML5 DOM widget creation for all GUI components: forms, buttons, labels, edits, panels, tabs, grids, canvas, etc. Form window management: titlebar, drag, minimize/maximize/close, z-index stacking, taskbar |
| `object_web` | `object_web.rs` (~1,200 lines) | Component creation, property storage, event dispatch. Central `rp_comp_create`, `rp_comp_get`, `rp_comp_set`, `rp_comp_method` API backed by thread-local `GUI_COMPONENTS` |
| `builtins_web` | `builtins_web.rs` | WASM-compatible built-in functions: string, math, I/O stubs, system functions |
| `datascience_web` | `datascience_web.rs` (~1,550 lines) | RNum (Vec<f64>), RDataFrame (column-oriented Vec<Vec<String>>), RPlot (HTML5 Canvas) |
| `database_web` | `database_web.rs` (~590 lines) | In-memory SQLite emulation: CREATE TABLE, INSERT, SELECT, UPDATE, DELETE, DROP TABLE with SQL parsing |
| `value` | `value.rs` | Shared `Value` enum (Int, Float, String, Null) for the web runtime |

### Web Build Pipeline

When `--web` is used:

1. **Codegen** emits Rust source targeting `rapidr-runtime-web` (different `use` paths, `#[wasm_bindgen(start)]` main function)
2. **Cargo** compiles to `wasm32-unknown-unknown` target in release mode
3. **wasm-bindgen** post-processes the `.wasm` to generate JS glue (`<name>.js`, `<name>_bg.wasm`)
4. **Index.html** is auto-generated to load the JS/WASM module
5. **Output** goes to `examples/<name>_web/` directory, ready to serve

```bash
cargo run -- --web examples/hello_web.rr
python3 -m http.server -d examples/hello_web_web 8080
```

### Asset Preloading & Embedding

Because browser-based WASM runtimes run synchronously on the main thread, synchronous file reads (e.g. `RDataFrame.loadfromcsv` or `RSQLite.connect`) cannot await asynchronous browser `fetch` calls. To resolve this:

1. During the web build or bundle process (`build_web` / `bundle_bc_file`), the compiler scans the directory of the source file for assets matching `.csv`, `.db`, `.sqlite`, `.png`, `.jpg`, `.jpeg`, `.gif`, `.bmp`, `.txt`, `.wav`, and `.mp3` extensions.
2. The compiler base64-encodes these assets and generates a `<script>` tag in `index.html` setting `window.__rapidr_assets = { ... }`.
3. In `rapidr-runtime-web`, file and database operations call `get_rapidr_asset()` to retrieve these base64-encoded strings, decoding them synchronously in-memory to match desktop capability.

### Form Window Management (Web)

Web `RForm` components behave like desktop windows:

- **Titlebar** — Flexbox layout with caption text span + minimize (−), maximize (□), close (✕) buttons
- **Drag-to-move** — `mousedown` on titlebar starts drag, `mousemove` updates `left`/`top`, `mouseup` ends drag
- **Z-index stacking** — Thread-local `FORM_Z_COUNTER` increments on each focus; `mousedown` on any form calls `form_bring_to_front()`
- **Minimize** — Hides form, creates a restore button in a fixed-position taskbar at viewport bottom
- **Maximize** — Saves current geometry in `FORM_SAVED_GEOMETRY` HashMap, sets form to viewport-filling dimensions; toggle restores
- **Close** — Sets `display: none`
- **Tab controls** — Tab buttons have click handlers calling `tab_switch()` which updates visual state and fires `onchange`

### Security Guidelines & WASM Safety

To maintain a secure sandbox within the web application:
- **No Dynamic Eval for System APIs**: Avoid using `js_sys::eval` or `new Function` for standard web-exclusive features (such as audio playback, notification display, or dynamic JavaScript execution).
- **Type-safe Audio**: Use `web_sys::HtmlAudioElement::new_with_src()` to load and play audio clips.
- **Type-safe Notifications**: Use `web_sys::Notification` and `web_sys::NotificationOptions` with appropriate string field mutators (e.g., `set_body`).
- **Reflect-based Interop**: Dynamic execution of global JavaScript helper functions (such as `RJAVASCRIPT.call`) must use type-safe property retrieval via `js_sys::Reflect::get` on the `window` object, followed by dynamic function application, preventing template-based JS injection.

### Web-Exclusive Components

9 components available only with `--web`:

| Component | HTML Element | Key Features |
|-----------|-------------|--------------|
| `RWebView` | `<iframe>` | `sethtml()`, `navigate(url)` |
| `RDOM` | any `<tag>` | `create()`, `setAttribute()`, `addClass()`, `querySelector()` |
| `RJavaScript` | (none) | `eval(code)`, `call(func, args)` |
| `RWebStorage` | (none) | localStorage/sessionStorage: `set`, `get`, `remove`, `clear`, `keys` |
| `RWebAudio` | `<audio>` | `play`, `pause`, `stop`, `seek` |
| `RWebVideo` | `<video>` | `play`, `pause`, `stop`, `seek`, `fullscreen` |
| `RWebNotification` | (none) | `requestpermission`, `show` |
| `RWebGeolocation` | (none) | `getposition` → latitude, longitude, accuracy |
| `RRouter` | (none) | SPA hash router: `navigate`, `back`, `forward`, `onroutechange` |

### Data Science on Web

The web runtime provides pure Rust + web-sys implementations (no ndarray, polars, or plotters):

- **RNum** — Backed by `Vec<f64>`. Supports arange, linspace, zeros, ones, fromlist, element-wise math (sin, cos, sqrt, exp, log, etc.), arithmetic (add, subtract, multiply, divide), aggregation (sum, mean, std, min, max, median), sorting, random generation, and more.
- **RDataFrame** — Column-oriented `Vec<Vec<String>>`. Supports addcolumn, setcell(col, row, val), cell(col, row), filter, sort, groupby, togrid, readcsv, and more. Auto-expands rows on setcell.
- **RPlot** — Renders to HTML5 Canvas. Supports line, bar, barh, scatter, step, area, histogram, pie charts, annotations, and legend.
- **RSQLite** — Full in-memory SQL emulation with CREATE TABLE, INSERT, SELECT (with WHERE, ORDER BY, LIMIT), UPDATE, DELETE, DROP TABLE. Methods: `connect`, `query`/`exec`, `fetchrow`, `fetchfield`, `row`.

### Multi-Form Programs

Both runtimes support any number of `RFORM` instances. Sibling forms behave like ordinary OS windows; nested forms (set `Parent="Form1"`) embed inside another form's client area:

```rapidr
$APPTYPE GUI

CREATE Form1 AS RFORM
    Caption = "Main"
    CREATE BtnOpen AS RBUTTON
        Caption = "Open Dialog"
        OnClick = OpenDlg
    END CREATE
END CREATE

CREATE Form2 AS RFORM
    Caption = "Dialog"
    CREATE BtnOK AS RBUTTON
        Caption = "OK"
        OnClick = CloseDlg
    END CREATE
END CREATE

SUB OpenDlg()
    Form2.ShowModal()        ' centred + dimmed backdrop on web
END SUB

SUB CloseDlg()
    Form2.Close()             ' fires OnClose, removes backdrop
END SUB

Form2.Hide()                  ' start hidden — survives gui_web_finalize
Form1.Show()
```

**Lifecycle methods on every form:** `Show`, `ShowModal`, `Hide`, `Close`, `Center`, `SetParent(name)`.
**Lifecycle events:** `OnLoad` (after all child widgets exist), `OnClose` (after Close/Hide).

For nested forms, just set the `Parent` property at create time:

```rapidr
CREATE Form2 AS RFORM
    Parent = "Form1"      ' embed inside Form1's client area
    Caption = "Inner"
END CREATE
```

### Web Build Cache Gotcha

After modifying source in `crates/rapidr-runtime-web/`, you must `cargo clean` inside each `examples/*_rust/` directory before rebuilding. Cargo's WASM cross-compilation target cache does not always detect path dependency changes:

```bash
# After changing runtime-web source
for d in examples/*_rust; do (cd "$d" && cargo clean); done
```

---

## 6c. Bytecode Interpreter (`rapidrintr`)

The bytecode pipeline lives under `interpreter/` and runs alongside the
existing Rust-codegen path. It compiles RapidR source to a compact
`.rrbc` artifact and executes it on a stack VM. The same `.rrbc` runs
on desktop (linked into the CLI) **or** in the browser (loaded by
`rapidrintr.wasm`) — no source recompile required for the web target.

### Crate Layout

| Crate | Purpose |
|-------|---------|
| `interpreter/rapidr-bytecode` | RRBC binary format, opcodes, hand-rolled (de)serialise |
| `interpreter/rapidr-vm` | Stack VM + `Host` trait (runtime-agnostic) |
| `interpreter/rapidr-bcgen` | AST → bytecode lowering |
| `interpreter/rapidr-vm-host-native` | `Host` impl backed by `rapidr-runtime-core` |
| `interpreter/rapidr-vm-host-web` | `Host` impl backed by `rapidr-runtime-web` (cdylib for wasm) |
| `interpreter/rapidr-compiler-wasm` | wasm-bindgen wrapper exposing `compile(src) -> Vec<u8>` |
| `interpreter/rapidr-webbundle` | Builds a static `.zip` (index.html + loader.js + wasm + rrbc) |

### Bytecode Format (`.rrbc`)

- Magic `RRBC` + `version: u16` (currently `1`).
- Constants pool (strings, integers, floats), function table, optional
  debug-info side-table.
- ~50 stack opcodes: `LOAD_CONST`, `LOAD_LOCAL`, `STORE_LOCAL`,
  `LOAD_GLOBAL`, `STORE_GLOBAL`, `ADD/SUB/MUL/DIV/MOD/POW/NEG`,
  `EQ/NE/LT/LE/GT/GE`, `AND/OR/NOT/XOR/BAND/BOR/BNOT/SHL/SHR`,
  `CONCAT`, `JUMP/JUMP_IF/JUMP_IFNOT`, `CALL_SUB/CALL_FUNC/RET/RET_VAL`,
  `CALL_BUILTIN`, `NEW_ARRAY/AGET/ASET/REDIM`,
  `CREATE_COMP/SET_PROP/GET_PROP/CALL_METHOD/REGISTER_EVENT`,
  `WITH_PUSH/WITH_POP`, `PRINT/INPUT`, `NULL/TRUE/FALSE`, `HALT`, `NOP`.

### Host Trait

Both runtimes implement the same `Host` surface:

```rust
pub trait Host {
    fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String>;
    fn create_comp(&mut self, kind: &str, id: &str) -> Result<Value, String>;
    fn set_prop(&mut self, id: &str, name: &str, value: Value) -> Result<(), String>;
    fn get_prop(&mut self, id: &str, name: &str) -> Result<Value, String>;
    fn call_method(&mut self, id: &str, method: &str, args: &[Value]) -> Result<Value, String>;
    fn register_event(&mut self, id: &str, event: &str, handler_fn_index: u32) -> Result<(), String>;
    fn print(&mut self, s: &str) -> Result<(), String>;
    fn input(&mut self) -> Result<String, String>;
}
```

Event re-entry uses an indirect-dispatch hook
(`EventHandler::Indirect(u32)` + a thread-local closure) so DOM/FLTK
callbacks invoke `Vm::invoke_function` on the parked VM instance.

### CLI

```sh
rapidr build-bc  hello.rr [-o hello.rrbc]                  # compile to .rrbc
rapidr run-bc    hello.rrbc                                # run via NativeHost
rapidr bundle-bc hello.rr [-o hello-web.zip]               # static web bundle
rapidr bundle-bc hello.rr --wasm path/to/rapidrintr.wasm \
                          --js   path/to/rapidrintr.js     # explicit artifacts
```

`bundle-bc` auto-locates the bytecode interpreter wasm/js under
`target/web/`, `target/web-bundle/`,
`interpreter/rapidr-vm-host-web/pkg/`, or `pkg/`. Override with
`--wasm`/`--js`. Bundle layout:

```text
<project>-web.zip
  index.html
  loader.js          (ES module: init wasm → fetch .rrbc → run_bc)
  rapidrintr.js
  rapidrintr.wasm
  <project>.rrbc
```

### Building the Web Artifacts

The recommended invocation is the helper script, which builds the
combined wasm (compiler + interpreter from a single cdylib) into
`target/web/`:

```sh
bash tools/build_web_artifacts.sh
# → target/web/rapidrintr.{js,wasm}
```

Under the hood that runs:

```sh
wasm-pack build interpreter/rapidr-vm-host-web --target web \
  --out-dir target/web --out-name rapidrintr --release
```

`rapidr-vm-host-web` was extended in April 2026 to depend on
`rapidr-lexer`, `rapidr-parser`, `rapidr-preprocessor`, and `rapidr-bcgen`
in addition to `rapidr-vm`/`rapidr-runtime-web`. It exports **two**
`#[wasm_bindgen]` entry points from the same module:

| Export | Signature | Purpose |
|--------|-----------|---------|
| `compile(source, project_name)` | `(String, String) → Result<Vec<u8>, JsValue>` | Preprocess → Lex → Parse → bcgen → bytes |
| `rapidr_run_bc(bytes)` | `(Vec<u8>) → Result<(), JsValue>` | Boot a `WebHost`, decode `Module::from_bytes`, run the VM |

This is what makes the self-contained web IDE possible — one wasm module
holds the entire compile + run loop. The standalone
`interpreter/rapidr-compiler-wasm` cdylib (compile-only) is still built
when needed for environments that just want a smaller compiler payload.

### Coexistence with the Rust-codegen Path

The bytecode pipeline is fully **opt-in**. All existing flags
(`rapidr file.rr`, `rapidr --web file.rr`, etc.) continue to invoke the
Rust-codegen pipeline unchanged. The `--interp` / `build-bc` / `run-bc` /
`bundle-bc` subcommands are the only entry points to the new path.

---

## 6d. Self-Hosted Web IDE (`web-ide/`) — v1.0.0

The repository ships a fully self-contained, zero-backend browser IDE
under `web-ide/`. It is plain HTML/JS — **not** a `.rr` program — and
drives the combined `rapidrintr.wasm` from §6c.

### Layout

```
web-ide/
  index.html      Tabbed shell — MDI tabs (Design / Code per form), Preview, Properties
  host.js         ~2100 LOC — boots wasm, renders UI, runs commands
  model.js        Pure project / form / widget model + serializer (.rr text)
  toolbox.js      TOOLBOX_GROUPS (3 groups, ~22 components) + isVisibleType()
  preview.html    Sandboxed runtime iframe (used for both Preview and design surface)
  zip.js          In-browser STORED PKZIP writer for the Build button
  ide.css         Theme + property-grid + design-tray styles
  vendor/monaco/  Monaco 0.52.2 (vendored MIT)
  runtime/        Symlink → ../target/web (output of tools/build_web_artifacts.sh)
```

### Architectural Choices

1. **One wasm, two roles.** `host.js` `import init, { compile, rapidr_run_bc }
   from "./runtime/rapidrintr.js"`, then `await init()` once. The Preview
   iframe also imports the same wasm. There is no second module.
2. **Iframe-as-runtime.** Both the Preview pane and each form's design
   surface are `<iframe src="./preview.html">`. The iframe self-announces
   readiness via `postMessage({__rapidr_preview_ready:true})` (or
   `__rapidr_design_ready:true` when launched with `?role=design`); the
   parent waits for that signal before posting `{__rapidr_run: bytes}`.
3. **WYSIWYG by construction.** The designer's "preview" *is* the
   runtime running the actual generated `.rr` source. There is no
   second renderer to maintain.
4. **Build = same bytes as `rapidr bundle-bc`.** `zip.js` writes a
   STORED-only PKZIP whose layout (`index.html`, `loader.js`,
   `rapidrintr.js`, `rapidrintr_bg.wasm`, `<name>.rrbc`, optional
   `assets/<name>`, `manifest.json`) mirrors
   `interpreter/rapidr-webbundle::build_bundle`. Bundled `index.html`
   ships a strict CSP meta tag.
5. **Multi-form, VB6-style.** Each `RForm` in the project gets its own
   pair of MDI tabs (`<form> [Design]` / `<form> [Code]`). Modules get
   a `[Module]` tab. The project tree drives the active form.

### Project Model (`model.js`)

```js
project = {
  name: string,
  forms: [{
    id, name,
    props: { caption, width, height, color, font, ... },
    children: [widget],
    code: { handlers: { OnClick: "Sub_Name", ... }, source: "..." }
  }],
  modules: [{ id, name, source }],
  assets: [{ name, mime, dataUrl }],     // base64 dataURLs, packed into bundle
  startupForm: id,
}

widget = {
  name, type,
  props: { left, top, width, height, ... },
  code: { handlers: { OnClick: "Button1_OnClick" } }   // sub-name binding
}
```

`serializeProject(project)` emits the full `.rr` source: `$APPTYPE WEB`,
module sources, one `CREATE … END CREATE` per form (with widgets nested
inside), a block of `Form.Event = SubName` bindings, the
`<startup>.ShowModal` line, then each form's `code.source`.

### Toolbox (`toolbox.js`)

```js
TOOLBOX_GROUPS = [
  { name: "Common Controls", items: [...visible widgets...] },
  { name: "Data & Web",      items: [...mixed visible + tray...] },
  { name: "I/O & Storage",   items: [...non-visual only...] },
];
TOOLBOX = TOOLBOX_GROUPS.flatMap(g => g.items);    // backwards compat
isVisibleType(type) → bool                          // tray vs form
defaultsFor(type)   → property defaults
```

Non-visual widgets (`RTimer`, `RHttp`, `RSqlite`, `RFileStream`,
`RWebStorage`, `RNum`, `RJson`, `RStringList`) drop into a dashed-border
tray *below* the form, are click-selectable, and get a property grid
just like visible ones.

### Property Grid

`renderProperties()` builds rows whose editor type is decided by
`propType(key)`:

| Type | Trigger | Editor |
|------|---------|--------|
| `enum`  | `widget.events[]` lookup or known enum prop | `<select>` |
| `asset` | key in `ASSET_PROPS` (`picture`, `dataset`, `csvfile`, …) | text + `<select project-assets>` + `+` button |
| `color` | key matches `/color\|background\|fill/` | text + native `<input type=color>` (live preview, OK confirms) |
| `font`  | key matches `/^font/` | dialog with family / size / weight / style |
| `bool`  | value is `true`/`false` | checkbox |
| `number`| `typeof value === "number"` | number input |
| `string`| else | text / textarea |

Every commit calls `setProp(...)`, re-runs `serializeProject`, and
re-renders the active design surface (debounced).

### Asset Pipeline

- *File → Upload Asset…* → `<input type=file>` → `FileReader.readAsDataURL`
  → push `{name, mime, dataUrl}` onto `state.project.assets`.
- *File → Manage Assets…* — list with sizes, individual remove.
- Asset properties auto-suggest from the project asset list.
- `serializeProjectModel` / `loadProjectModel` round-trip assets in JSON.
- `buildBundleZip({…, assets, version})` decodes each dataURL, writes
  `assets/<name>` as STORED (no recompression of pre-compressed
  formats), and emits `manifest.json`:
  ```json
  { "rapidr_bundle": 1, "project": "...", "title": "...",
    "ide_version": "1.0.0", "built_at": "...", "asset_count": N }
  ```

### Security Posture

- All user-controlled strings interpolated into IDE DOM (form names,
  module names, widget names in `<option>`) are run through
  `escapeHtml()`. Property-grid values use `value=…` on real inputs,
  not raw HTML.
- No `eval()` or `new Function()` outside vendored Monaco +
  wasm-bindgen glue.
- Bundled `index.html` ships:
  ```
  Content-Security-Policy: default-src 'self';
    script-src 'self' 'wasm-unsafe-eval';
    style-src 'self' 'unsafe-inline';
    img-src 'self' data: blob:;
    font-src 'self' data:;
    connect-src 'self';
    frame-src 'none'; object-src 'none'; base-uri 'self';
  ```
- Preview iframe is sandboxed (`allow-scripts allow-same-origin
  allow-modals`).
- Third-party attribution: [`LICENSES.md`](LICENSES.md).

### Test Coverage

| Test | Asserts |
|------|---------|
| `tests/web_ide_phaseF.mjs` | 12 — boot, smoke, regenerate |
| `tests/web_ide_bugfixes.mjs` | 16 — long-tail regression sweep |
| `tests/web_ide_round3.mjs` | 18 — multi-form, paste, About, modules |
| `tests/web_ide_round4.mjs` | 11 — color/font realtime + OK, Build zip wires runtime |
| `tests/web_ide_assets.mjs` | 6 — upload → propgrid → JSON round-trip → zip entry |
| `tests/web_ide_e2e_build.mjs` | full E2E — drive IDE → Build → unzip → spawn 2nd HTTP server → click button → assert label updates |
| `tests/web_smoke.mjs` | In-browser `compile()` ≡ native CLI for `hello_world.rr` |
| `tests/web_matrix.mjs` | `rapidr bundle-bc` for every web example, served + opened in Chromium, no console errors |

All eight (plus `tests/full_matrix.sh`) are kept green as part of the
1.0 release checklist.

### Subtle Bugs Already Found and Fixed

- **Status bar clobber.** Design-surface iframe was overwriting the
  parent's "ready" status. Fix: only the real Preview drives the
  parent status bar.
- **Iframe `load` race.** Parent posted bytecode before iframe's
  top-level `await init()` resolved. Fix: iframe posts
  `__rapidr_preview_ready` only after init resolves; parent waits.
- **Design-surface state pile-up.** Without iframe reload, components
  stacked. Fix: `cloneNode` the iframe element on every render.
- **Built-zip ESM loader mismatch.** Loader `import init from
  "./rapidrintr.js"` but the wasm-bindgen glue exports `__wbg_init` as
  default. Fix: loader uses `import { __wbg_init as default } from …`
  pattern; verified by Round-4 zip-byte test.
- **Color/font picker discarding edits.** Native pickers fire `input`
  → live preview only; `change` → commit. OK button now finalizes.
- **XSS via crafted form name.** Loading a `.rrproj` whose form name
  contained `<script>` was previously injected raw into `innerHTML`.
  Fix: `escapeHtml()` on every interpolation site.

### Legacy Web IDE (`examples/web_ide.rr` + `rapidr-buildserver`)

The earlier flow — a `.rr` IDE compiled to wasm via the Rust-codegen
path, talking to a local `rapidr-buildserver` HTTP service for compile /
preview / export — still works and the crate is kept in the workspace
for backwards compatibility. New work should target `web-ide/` instead;
it has no server requirement, ships in a single static-file directory,
and shares its compiler/runtime with `rapidr bundle-bc` outputs.

---

## 7. Code Generation Patterns

### Rust Code Generation (RapidR)

The Rust codegen (`rapidr-codegen-rust`) generates a standalone Rust project with `Cargo.toml` and `src/main.rs`. Generated code targets `rapidr-runtime-core` (native) or `rapidr-runtime-web` (WASM).

#### Global Variable Mechanism

Module-level `DIM` variables are stored in thread-local `HashMap` storage, accessed via helper functions:

```basic
DIM Counter AS INTEGER          → gs("counter", v_int(0));     ' Initialize
Counter = Counter + 1           → gs("counter", &gv("counter") + &v_int(1));  ' Assign
PRINT Counter                   → rp_print(&[gv("counter")], true);  ' Read
```

**Accessor functions:**
- `gs(name, value)` — Set a global scalar
- `gv(name)` — Get a global scalar (returns owned `Value`)
- `ga_init(name, size, default)` — Initialize a global array
- `ga_get(name, index)` — Get global array element
- `ga_set(name, index, value)` — Set global array element

**UDT variables** at module level remain as native Rust structs (not stored in the HashMap), preserving field access syntax.

#### How a DIM becomes Rust
```basic
DIM x AS INTEGER        →  gs("x", v_int(0));              ' Global (top-level)
DIM x AS INTEGER        →  let mut x = v_int(0);           ' Local (inside SUB/FUNCTION)
DIM s AS STRING         →  gs("s", v_str(""));             ' Global string
DIM f AS RForm          →  rp_comp_create("f", "RFORM");   ' GUI component
DIM a(10) AS DOUBLE     →  ga_init("a", 11, v_dbl(0.0));   ' Global array (0-10 = 11 elements)
DIM p AS PersonType     →  let mut p = PersonType::default(); ' UDT instance
```

#### How a CREATE block becomes Rust
```basic
CREATE Form1 AS RForm           →  rp_comp_create("form1", "RFORM");
    Caption = "Test"            →  rp_comp_set("form1", "caption", v_str("Test"));
    CREATE Btn AS RButton       →  rp_comp_create("btn", "RBUTTON");
        Parent = Form1          →  rp_comp_set("btn", "parent", v_str("form1"));
        Caption = "OK"          →  rp_comp_set("btn", "caption", v_str("OK"));
    END CREATE
END CREATE
```

#### How a SUB becomes Rust
```basic
SUB Foo(x AS STRING)            →  fn foo(x: Value) {
    DIM local AS STRING         →      let mut local = v_str("");
    local = "hello"             →      local = v_str("hello");
    PRINT local                 →      rp_print(&[local.clone()], true);
END SUB                         →  }
```
Global variables are accessed via `gv()`/`gs()` — no `global` declarations needed (thread-local storage handles sharing).

---

## Changelog (April 2026)

### Parser Fixes
- **Dot-member access after keywords**: `parse_postfix_expression` and `parse_primary` (WITH-dot case) now accept ANY token after `.`, not just `Identifier`. This fixes `Form1.Close`, `Form1.Show`, `ListBox1.Clear`, etc., where the member name is also a keyword.

### Codegen Fixes
- **RTIMER registration**: `emit_create()` now emits `gui_register_timer("{name}")` after the CREATE block body for RTIMER components.

### Runtime GUI Fixes (`gui.rs`)
- **Visibility default**: Components without an explicit `visible` property are now visible by default. Previously, missing/null `visible` was treated as 0 (hidden), making RListBox, RCanvas, RImage, etc. invisible. Now only explicitly set `"false"` or `"0"` hides a widget.
- **RButton visual feedback**: Replaced FrameType-based hover/press effects (invisible with themes) with color-based feedback. Added Focus/Unfocus/KeyDown handlers for keyboard interaction.
- **RCanvas inline color**: All drawing methods (`line`, `rect`, `fillrect`, `circle`, `ellipse`, `arc`, `drawtext`) accept an optional trailing color argument.
- **RCanvas drawtext**: Supports dual argument conventions — `drawtext(text, x, y)` and `drawtext(x, y, text)`.
- **RCanvas paint method**: Now calls `redraw_widget` instead of delegating to `fillrect`.
- **RCanvas onclick event**: Push handler now fires `rp_fire_event(name, "onclick")`.
- **Menu duplication fix**: RMENUITEM entries that have children (submenu headers like "&File") are no longer added to the MenuBar via `mb.add()`. Their children's full-path entries (e.g., `"&File/&New"`) auto-create the submenu, preventing duplicate top-level menu entries.
- **RDESIGNSURFACE positioning**: Reads `left`/`top` from component properties instead of hardcoding (200, 200), enabling embedded MDI-style placement inside parent forms.
- **Component idempotency**: `rp_create_component` and `gui_create_widget` skip creation if the component/widget already exists, preventing duplicate widgets from DIM+CREATE patterns.

### IDE Example (`ide.rr`)
- **MDI layout**: DesignSurface is now created inside the IDE form's CREATE block with `Left=4, Top=124, Width=832, Height=580`, embedding it directly in the center area rather than as a separate floating window.
- **Import `Key`**: Added `fltk::enums::Key` to imports for button keyboard handling.
- **Code preservation (v1.1)**: GenerateCode uses `[AUTO-DECLARATIONS]`/`[AUTO-FORM]` markers. User code (globals, constants, FUNCTIONs, SUBs) between the markers is preserved across regenerations.
- **Improved compilation (v1.1)**: CompileAndRun now captures build output to a log file and displays errors line-by-line in the ErrorList.
- **RCoolBtn/ROvalBtn in toolbox (v1.1)**: Added "Cool" and "Oval" buttons to the component palette with full property/event support.
- **Open in VS Code (v1.1)**: Tools → "Open in VS Code" shells out to `code` to open the project file externally.

### Menu System Fixes
- **SysMenuBar**: `RMAINMENU` now uses FLTK `SysMenuBar` instead of `MenuBar`. On macOS, this renders the menu in the native system menu bar. On other platforms, it acts as a regular in-window menu bar.
- **Menu y-offset**: When a form has an `RMAINMENU` child, all non-menu child widgets are automatically offset by 30px on non-macOS platforms to prevent overlapping with the in-window menu bar.
- **REDIT text sync**: `rp_comp_get` for REDIT components now reads the `text` property directly from the FLTK Input widget, not from the cached property registry. This fixes the demo_mysql Edit button bug where typed text wasn't returned because the Input callback only fires on Enter.

### New Components
- **RCOOLBTN**: Flat/toggle toolbar-style button. Properties: Caption, Flat, GroupIndex, Down, AllowAllUp, BMP, NumBMPs, Layout, Spacing. Uses FLTK flat button with custom toggle/group logic. On web: CSS-styled `<button>` with optional image.
- **ROVALBTN**: Oval/round button with custom draw. Properties: Color, ColorHighlight, ColorShadow, Flat, GroupIndex, Down. Uses `draw_pie`/`draw_arc` for 3D oval appearance. On web: CSS `border-radius: 50%` button.

### CLI
- The `rapidr` binary can be installed globally via `cargo install --path crates/rapidr-cli`.

### Runtime GUI Fixes (April 7, 2026)
- **StringGrid clear corruption fix**: `scroll.clear()` was destroying FLTK's internal scrollbar children (scrollbar and hscrollbar), corrupting the Scroll container and making subsequently added widgets non-interactive. Replaced with a safe `while scroll.children() > 2 { scroll.remove_by_index(0); }` loop that preserves the 2 internal scrollbar widgets. Also resets `selected_col` and scroll position on clear.
- **RFORM top-level window fix**: Added `Group::set_current(None::<&Group>)` before creating non-child RFORM windows in `gui_create_widget`. Without this, FLTK's current group context could cause new forms (e.g., dialog windows opened at runtime) to become embedded child widgets instead of top-level windows.
- **OnShow event**: `gui_showmodal` and `gui_show` now fire `rp_fire_event(name, "onshow")` after building form widgets and calling `win.show()`. This enables post-build initialization (e.g., populating widget text after the widget is guaranteed to exist).
- **RPlot in-memory rendering**: Plot rendering refactored from file-based to fully in-memory. `render_plot_bytes()` uses `BitMapBackend::with_buffer()` to render into an RGB pixel buffer, then `encode_rgb_to_png()` encodes to PNG bytes via the `image` crate. `loadfromplot` in gui.rs now calls `PngImage::from_data(&bytes)` instead of writing/reading temp files. This eliminates FLTK's `SharedImage` cache stale-image bug and removes all temporary file I/O from the plot→image pipeline. The `savefig`/`save` method still writes to disk via `std::fs::write()` of the same bytes.

### IDE Example Fixes (April 7, 2026)
- **Event Editor dialog**: Added EventEditorDlg form with `edtEvtCode` (RCodeEditor), OK/Cancel buttons, and `EventEditorOnShow` handler. The "..." column in EvtGrid opens the event editor dialog.
- **Event code storage**: Added `GetEventCodeBody`, `SetEventCodeBody`, `SyncEventCodeFromSource` functions and updated all 34 `GenerateSubCode` callers to use stored event code bodies from arrays.
- **EditorTempCode + OnShow pattern**: Added `EditorTempCode` global variable. `OpenEventEditor` sets it before `ShowModal`; `EventEditorOnShow` force-sets `edtEvtCode.Text = EditorTempCode` after widgets are guaranteed to exist, fixing text not appearing on second invocation.
- **DIM removed for visual components**: `GenerateCode` in ide.rr no longer emits `DIM` for visual components (RFORM, RBUTTON, etc.). Only non-visual dialog types (ROPENDIALOG, RSAVEDIALOG, RCOLORDIALOG, RFONTDIALOG) get `DIM` declarations.

### Web Runtime Fixes (April 7, 2026)
- **gui_register_timer signature**: Fixed `gui_register_timer` in `object_web.rs` from 2-arg `(_name: &str, _interval: i32)` to 1-arg `(_name: &str)` to match desktop runtime and codegen.
- **web_datascience.rr RNUM declarations**: Added missing `DIM arr1 AS RNum` and `DIM arr2 AS RNum` declarations. Without these, the codegen couldn't identify them as RNUM components and fell back to `rp_comp_get(...)()` (property-get + call) instead of `rp_comp_method(...)`.

### Dependencies (April 7, 2026)
- **image crate**: Added `image = { version = "0.24", optional = true, default-features = false, features = ["png"] }` to `rapidr-runtime-core` Cargo.toml, gated under the `datascience` feature. Used for in-memory PNG encoding of plot renders.

### Demo Updates (April 7, 2026)
- **demo_plot.rr**: Added "Pie Chart" button (`BtnPieClick`) that renders a "Browser Market Share" pie chart with 5 labeled/colored slices using `plt.pie`. Buttons rearranged: Sine/Cosine, Bar Chart, Pie Chart, Clear Plot, Close.

### Multi-Form Authoring + Web Build Server (April 23, 2026)
- **Multi-form web runtime**: `gui_web_finalize` now correctly handles multiple top-level forms — each top-level form gets its own stacking z-index (10, 11, 12...), orphan widgets are reparented only into the first top-level form (forms manage their own placement), and `data-rr-name` / `data-rr-parent` attributes were added to every form element so the finalize routing is unambiguous. Honors the `visible` property: pre-startup `Form.Hide()` calls now actually keep the form hidden on first paint.
- **Form lifecycle events**: `OnLoad` is now fired exactly once per form after all child widgets have been built (both desktop `build_form_widgets` and web `gui_web_finalize`). `OnClose` is fired by `gui_close` on desktop and by `(_, "close")` / `form_close` on web — symmetric across runtimes.
- **`Form.ShowModal()` on web**: Implemented as a pseudo-modal — adds a dimmed full-screen backdrop overlay, brings the form to the front, and centres it on screen. Backdrop is removed automatically on `Hide`/`Close`. True blocking is impossible in single-threaded wasm; document this in user-facing code.
- **Nested forms (`Parent="Form1"`)**: A form whose `Parent` property names another form is now appended to that form's `-client` div on web (via the rewritten `create_form` helper). Desktop honours the same `parent` property when building children. A new `SetParent(formName)` runtime method is exposed on every component for late re-parenting (web-only at runtime; desktop sets the prop but does not yet reparent live FLTK widgets).
- **`RWEBVIEW.SetHtml` / `html=` on web**: Now correctly uses `iframe.set_srcdoc()` instead of the no-op `set_inner_html` on iframe elements.
- **Listbox `Clear` no-loop fix**: Replaced `select.remove()` (inherited from `Element` and removes the element itself) with `select.remove_with_index(0)` — fixes an infinite loop that froze the IDE.
- **`rapidr-buildserver` crate**: New axum-based HTTP service used by the IDE to compile, preview, and download programs. Endpoints:
  - `POST /compile` — body is RapidR source. Runs `./rapidr --web` and (best-effort) a native `cargo build --release`. Returns `{ id, ok, stderr, preview, zip_source, zip_full }`.
  - `GET  /preview/<id>/`  — serves the compiled web bundle (HTML/JS/WASM) so the IDE's `RWEBVIEW Preview.Navigate(...)` can show the running program.
  - `GET  /zip/<id>/source` — `rapidr-source.zip` containing the `.rr` source.
  - `GET  /zip/<id>/full`   — `rapidr-bundle.zip` containing source + web bundle + native binary (when build succeeded).
  Launch with `./run_buildserver.sh` (defaults to port 8095, override with `RAPIDR_BUILDSERVER_PORT`).
- **Web IDE Run/Preview/Export**: `examples/web_ide.rr` gained four new toolbar buttons:
  - **▶ Run** — POSTs the generated source to the build server and points the embedded `Preview` `RWEBVIEW` at `/preview/<id>/`.
  - **Zip Src** — downloads `/zip/<id>/source`.
  - **Bundle** — downloads `/zip/<id>/full`.
  - **Design** — switches the centre pane back to the canvas.
  The IDE expects the build server at `http://127.0.0.1:8095` (override the `BUILDSERVER` constant near the top of `web_ide.rr` if needed).
- **`RHTTP.Post` body escaping**: The XHR body is now properly escaped for embedding in the synchronous-eval JS string (backslash, quote, CR, LF, tab) so multi-line bodies (such as RapidR source code) round-trip correctly.
- **Web IDE designer enhancements** (`examples/web_ide.rr` + `crates/rapidr-runtime-web/src/gui_web.rs`):
  - **Form events** — selecting a form in the designer now lists `OnShow / OnClose / OnClick / OnResize`. **Edit Event Code** opens an inline editor that emits a `SUB Form_OnEvt()` and the matching `Form.OnEvt = Form_OnEvt` wiring.
  - **Color / TextColor / FontName / FontSize properties** — added to the property grid for both forms and components. The codegen emits them inside the `CREATE … END CREATE` block only when non-empty (`Color = RGB(80,200,120)`, `FontName = "Verdana"`, `FontSize = 16`, etc.).
  - **Editable property grid** — `grid_init_cells` and `grid_set_row_count` in the web runtime now mark non-header value cells as `contenteditable="true"`, so users can click any property cell, type a new value, and click **Apply Properties**.
  - **Color & font pickers + live apply** — the IDE injects a small JS helper at startup that (a) renders a `…` button next to each Color / TextColor / FontName / FontSize row in the property grid, opening either a native `<input type="color">` or a custom font dialog (family list + size + live preview); and (b) debounces an auto-Apply on every `input` event in the property grid so caption / position / colour / font changes redraw the designer canvas in real time. The font dialog writes both the FontName and FontSize rows. The IDE's `DrawForm` parses the stored `RGB(r,g,b)` string via a new `ParseRgb` helper and uses it for the form body fill, title text colour, and (via the new `RCANVAS.SetFont` runtime method) the title font.
  - **`RCANVAS.SetFont(name [, size])`** — new web-runtime canvas method that sets `ctx.font = "<size>px <family>"` so subsequent `DrawText` calls render in the chosen font. Used by the IDE designer to reflect form FontName/FontSize visually.
  - **Canvas text baseline fix** — `canvas_text` in the web runtime now sets `ctx.set_text_baseline("top")` before `fill_text`, matching desktop FLTK semantics where `y` is the top of the text. Fixes form-titlebar text being clipped above the canvas in the IDE designer (and any RCANVAS app that draws text).
  - **Build cache** — `rapidr-buildserver` keeps an in-memory map of `sha256(source) → build_id`, so re-running an unchanged program serves the cached WASM in milliseconds instead of recompiling.
  - **Busy overlay** — the IDE shows a translucent "Compiling…" overlay during the synchronous build XHR so users get visual feedback that the click registered.

- **VS Code extension v2.7.0** (`utilities/vscodeext/rapidr/`):
  - `RCANVAS.SetFont(family [, size])` is now in the component registry — completion, hover docs, and signature help include it.
  - `RColorDialog` and `RFontDialog` now ship with descriptions and `Execute()` signature help. Two new snippets — `createcolordialog` and `createfontdialog` — scaffold the full `CREATE … END CREATE` + `IF Dlg.Execute() THEN …` pattern.
  - New `canvassetfont` snippet pairs `Canvas.SetFont` with a `Canvas.DrawText` call.
  - Build with `./build_vsc_extension.sh package` (or `install` to also load it into VS Code). The script writes `rapidr-2.7.0.vsix` to the repo root.

---

## 8. Known Limitations & Gotchas

### Language Limitations
1. **No `[`, `]`, `{`, `}` in source code.** The lexer rejects these characters. Workaround: use component methods (RNum.arange, RDataFrame.loadfromcsv) or `RUSTSTART`/`RUSTEND` blocks for raw Rust.
2. **`LEN()` function** converts to string first — `LEN(x)` returns the length of the string representation.
3. **Array/function ambiguity.** `foo(i)` in expressions is always parsed as a function call. The codegen resolves it via symbol table lookup. If a variable isn't tracked (e.g., returned from a method, stored in VARIANT), it may be incorrectly emitted as a function call.
4. **All identifiers are lowercased.** `MyVar` and `MYVAR` are the same variable.
5. **No multi-line string literals.** Strings must be on a single line.
6. **No lambda/anonymous functions.** Use `CODEPTR(SubName)` for function references.
7. **GOTO/GOSUB recognized but not implemented** in codegen.
8. **Single `=` for both assignment and comparison.** Context determines meaning (statement = assignment, expression = comparison).

### Compiler Gotchas (Rust Codegen)
1. **`is_component_type_name()`** and **`is_component_method_name()`** in `codegen-rust/src/lib.rs` must be updated when adding new component types or methods.
2. **Pre-pass ordering matters.** UDTs are collected first, then globals/arrays/consts, then SUB/FUNCTION signatures, then directives. Changing this order can break symbol resolution.
3. **String suffix stripping** — `LEFT$()` becomes `left`, `STR$()` becomes `str_func`.

### Runtime Gotchas (Rust)
1. **FLTK event loop.** `gui_showmodal()` runs `app.run()` which blocks. The first form shown should be the main form.
2. **Thread-local storage.** All global variables and component state use `thread_local!`. Cross-thread GUI access is not supported.
3. **RStringGrid.AddRow** — takes variable positional string args. Column count must be set first.

---

## 9. How to Add New Features

### Adding a New GUI Component (Rust)
1. **`crates/rapidr-runtime-core/src/gui.rs`** — Add widget creation in `gui_create_widget()` match arm
2. **`crates/rapidr-runtime-core/src/object.rs`** — Add default properties in `rp_comp_create()` match arm
3. **`crates/rapidr-codegen-rust/src/lib.rs`** — Add to `is_component_type_name()` and `is_component_method_name()`
4. **`crates/rapidr-runtime-web/src/gui_web.rs`** — Add DOM element creation for the web target
5. **`crates/rapidr-runtime-web/src/object_web.rs`** — Add default properties and method dispatch for web
6. **Add tests** in the appropriate crate

### Adding a New Builtin Function (Rust)
1. **`crates/rapidr-runtime-core/src/builtins.rs`** — Implement the function
2. **`crates/rapidr-codegen-rust/src/lib.rs`** — Add to the builtin function emission logic
3. **Add tests**

### Adding a New Keyword
1. **`crates/rapidr-lexer/src/lib.rs`** — Add to `TokenKind` enum and keyword matching
2. **`crates/rapidr-parser/src/lib.rs`** — Add parsing logic
3. **`crates/rapidr-ast/src/lib.rs`** — Create new AST node if needed
4. **`crates/rapidr-codegen-rust/src/lib.rs`** — Add code emission logic

### Adding a New Preprocessor Directive
1. **`crates/rapidr-preprocessor/src/lib.rs`** — Add handling in the directive processing loop

---

## 10. Test Suite

### Rust Tests

```bash
cargo test
```

| Crate | Tests | Coverage |
|-------|-------|----------|
| `rapidr-codegen-rust` | 6 | DIM, FOR, assignment, expression codegen |
| `rapidr-lexer` | 7 | Token types, keywords, operators, literals |
| `rapidr-preprocessor` | 1 | Directive handling |
| `rapidr-parser` | 15 | AST generation for all language constructs |
| `rapidr-diagnostics` | 1 | Span and diagnostic types |
| `rapidr-ast` | 9 | AST node validation |
| `rapidr-cli` | 31 | CLI commands, example compilation smoke tests |

---

*End of manual. Keep this file updated as the compiler evolves.*

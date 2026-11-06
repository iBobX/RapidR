# Big numbers: the audit (BIG-NUMBERS-A, 2026-11-05)

Every place a number goes, checked for INT64, UINT64 and UINT32: what it
did in 2.122.1 (and on the development build of 2.123 before this lane),
what it does now, and the test that holds it. The design is
[big-numbers-design.md](big-numbers-design.md): the value carries its type
(`Value::Int64` / `Value::UInt64`), so most surfaces were fixed once, in
`rapidr-value`'s generic operations, for every runtime.

The rule (Robert, 2026-11-05): big numbers are compatible with everything.
Nothing is silently cut or turned into a DOUBLE; a surface that can't hold
a value says so with a clear error. "Every runtime" below means the
interpreter with the JIT off and with `RAPIDR_JIT=always`, native builds,
and the web (the same VM and `rapidr-value` in wasm), unless a row says
otherwise.

Tests named below:
- **promised**: `tests/conformance/cases/big_numbers_promised.rr`
- **input**: `tests/conformance/cases/big_numbers_input.rr` (+ `.input`)
- **everywhere**: `tests/conformance/cases/big_numbers_everywhere.rr`
- **int64**: `tests/conformance/cases/int64_uint64.rr` (2.122.1's, kept)
- **classic**: `tests/conformance/cases/big_numbers_classic.bas` (a RapidQ
  program: its expected output is RC.EXE's, probe 2026-11-05)
- **web input**: `tests/web_bundle_console.mjs` (INPUT into INT64 / UINT64
  in a web bundle, typed with real keys)
- **jit**: `interpreter/rapidr-jit/tests/same_as_interpreter.rs`
  (`sixty_four_bit_loops_compiled`) and `edge_values.rs` (the 64-bit edge
  values through every compiled operation)
- **example**: `examples/basics/bits_and_64bit.rr` in `tests/examples_run.mjs`
- **unit**: `cargo test` of the module named

The conformance cases run on the interpreter (`vm`, `vm-jit`), native
(`codegen`) and the web (`tests/web_conformance.mjs`; the INPUT case on the
web through **web input**).

## 1. The five promised items (groups.io #38940)

| Item | Before | Now | Test |
|---|---|---|---|
| FOR loop's end test | through a DOUBLE: `FOR i = 2^53+1 TO 2^53+3` ran 5 times; a loop to a type's last value, and `FOR u = 10 TO 0 STEP -1` on a UINT64, never ended | exact compare; a 64-bit counter's step past its type's end ends the loop, the counter as it was (VM ForStep, native `rp_for_next`, the JIT's compiled step) | promised, jit, unit `wide::for_steps_end_at_the_type` |
| SELECT CASE on INT64 / UINT64 | through a DOUBLE: `CASE 9007199254740993` never matched, a UINT64 above 2^63 matched nothing | exact `=`, ranges and `IS`; the CASE literals read whole | promised |
| INPUT into INT64 / UINT64 | the typed line read as a DOUBLE, then stored | read exactly (`wide::input`); UINT32 too | input, web input |
| FORMAT$ `%d` / `%x` | 32 bits: -2147483648 / 80000000 | every digit of a 64-bit value, a precision up to 20; `%f` / `%n` of one exact too (a classic number keeps RapidQ's 32 bits) | promised |
| `&` and `+` with text | a UINT64 above 2^63 showed negative | its unsigned digits | promised, everywhere |

2.122.1's known-limits line is gone from its release notes; CHANGELOG's
Unreleased says what was fixed.

## 2. Values and the language

| Surface | Before | Now | Test |
|---|---|---|---|
| A VARIANT holding one | cut to 32 bits (-2147483648), a UINT64 negative | the 64-bit value | everywhere, input |
| VARIANT arrays, untyped parameters, a FUNCTION returning its argument | cut / negative | kept | everywhere |
| Typed arrays (`AS INT64`, `AS UINT64`), native typed arrays | right when the compiler saw them; a native UINT64 typed array read back signed | kept; native UINT64 arrays are `u64` | int64, everywhere |
| TYPE fields | right in expressions; the field's default was a classic 0 | kept; defaults typed | int64, everywhere |
| `SUB` / `FUNCTION` 64-bit parameters | right | right; literal arguments read whole | everywhere |
| A 64-bit FUNCTION's result | not converted (`F = 3000000000` returned a classic number: PRINT -2147483648) | converted to its type | everywhere (`FromLiteral`) |
| `DIM` defaults | a classic 0 | a typed 0 (`Const::Int64` / `UInt64`) | everywhere |
| `CONST x AS INT64` | read as wide only where the compiler saw it | the value of its type | int64, everywhere |
| `DATA` / `READ` | a decimal beyond 2^63 was a DOUBLE: READ into a UINT64 gave 0; an INT64 past 32 bits into a VARIANT cut | whole (UINT64 above 2^63) in a program with 64-bit types | everywhere |
| Literals beside a 64-bit / UINT32 value | `&H` up to 8 digits read as RapidQ's signed 32 bits: `i AND &HFFFFFFFF` kept every bit, `u32 = &HF58ACBFB` comparisons false | read as written (unsigned), up to 16 digits | everywhere (`literals`), example |
| A literal beyond 32 bits in a program with 64-bit types | a classic number: `PRINT 3000000000` -2147483648, `3000000000 \ 2` -1073741824 | an INT64 (a UINT64 above 2^63) | everywhere (`literal`), int64 |
| A decimal beyond 64 bits in a classic program | **text** (`v = 18446744073709551615 : PRINT v + 1` joined) | a DOUBLE, as RC.EXE reads it | classic (RC.EXE's output) |
| A classic program | — | unchanged: RapidQ's 32-bit rules (no 64-bit kind is ever made) | classic, the whole conformance suite |
| `+ - *` | wrapped at 64 bits where seen | wrap at 64 bits everywhere; UINT64 wins; a DOUBLE makes a DOUBLE | int64, everywhere, unit `wide::operates` |
| `\`, MOD | exact where seen | exact everywhere; by zero "Division by zero" | int64, everywhere |
| Comparisons | exact where seen; against a DOUBLE through floats | exact everywhere, against a DOUBLE too (2^53 + 1 > 2^53.0) | everywhere (`against a DOUBLE`), unit |
| AND OR XOR NOT SHL SHR | 64-bit where seen | 64-bit everywhere | int64, everywhere |
| Unary minus, ABS | ABS(min) wrapped (0f464de2) | both wrap, as RC.EXE's LONG gives `ABS(-2147483648)` stored back: -2147483648 (probe 2026-11-05). Decided: no overflow error (consistent with `+ - *`, which hashes need) | everywhere, unit |
| SWAP, INC / DEC, IIF | through VARIANT copies: kept | kept | everywhere |
| QUICKSORT | compared numbers as DOUBLEs (2^53 and 2^53 + 1 equal) | exact for 64-bit values | everywhere (`sorted`) |
| VARTYPE, ISNUMERIC | 2 (an Integer) | 20 (INT64, VT_I8) / 21 (UINT64, VT_UI8); numeric | everywhere |
| UINT32 | "Unknown data type UINT32" | 0 … 4294967295, held as an INT64 value | input, everywhere, example |
| DWORD (Jango's notes) | `w = 4119514107` → -2147483648, `&HF58ACBFB` → -175453189 | **unchanged: RC.EXE does exactly that** (probe 2026-11-05: -2147483648, -175453189; HEX$ of both F58ACBFB); UINT32 is the unsigned type | classic, ground truth |
| `PRINT 1, 2` → `12` (Jango) | — | **unchanged: RC.EXE prints `12`** (PRINT's comma is its semicolon; docs/rapidq-ground-truth.md) | `print_separators` |

## 3. Builtins

| Builtin | Before | Now | Test |
|---|---|---|---|
| STR$ | full where seen | full everywhere | everywhere |
| HEX$ | 16 digits always for a 64-bit value | 8 digits, 16 when the number needs more (UINT32 4119514107 is F58ACBFB, as Jango expected); `HEX$(x, digits)` at least `digits`, never cut | everywhere, int64, registry example |
| BIN$, OCT$ | 32 bits (OCT$ of a classic negative 64) | a 64-bit value's 64 bits; classic unchanged | everywhere |
| VAL | a DOUBLE (an i64-sized whole number exact as an integer) | unchanged (RapidQ's); `u = VAL(s$)` into a 64-bit variable reads the text whole | everywhere (`VAL`) |
| ABS, INT, FIX, ROUND, CEIL, FLOOR | DOUBLE / 32-bit | a 64-bit value is its own | everywhere |
| CINT, CLNG | 32-bit | 32-bit (their definition) | — (explained) |
| SGN | right | right | everywhere |
| SQR, SIN, COS, TAN, ATN, LOG, EXP, `^`, `/` | DOUBLE | DOUBLE: their results are real numbers (explained, not a cut) | — |
| SHL, SHR (functions) | 32-bit on a classic number | a 64-bit value on 64 bits | int64 |
| LOWORD, HIWORD, LOBYTE, HIBYTE, GETBITS, SETBITS | — (Jango, #38944) | new; a range outside the value's bits is an error naming it | everywhere, example, unit `wide::bit_functions`, registry examples (`rapidr lang truth`) |
| FORMAT$ `%e %g %m` of a 64-bit value | DOUBLE | DOUBLE (scientific and money forms are approximate by nature; `%d`, `%x`, `%f`, `%n` are exact) | — (explained) |
| HEXTODEC | 0 for 9 to 16 digits | unchanged (classic; `u = "&H…"` isn't a number in RapidQ either) — open: a 64-bit HEXTODEC is a small follow-up | — |
| FILELEN, LOF | a file past 2 GB showed -2147483648 | its size whole (an INT64 beyond 32 bits) | unit (`file_io`), manual |

## 4. Files, streams, memory, DLLs

| Surface | Before | Now | Test |
|---|---|---|---|
| PRINT #, WRITE # | full where seen | full | everywhere (`file`) |
| INPUT #, LINE INPUT # into a 64-bit variable | through the store: text read whole | whole | everywhere |
| Streams: `WriteNum` / `ReadNum` | no 64-bit integer kind (Num_DOUBLE rounded) | `Num_INT64` (9), `Num_UINT64` (10), `Num_UINT32` (11): 8 / 4 bytes, little-endian; `LoadArray` with them | unit (memstream), manual |
| Streams: `Write v` / `Read v` of a 64-bit variable | 4 bytes (cut) | 8 bytes. A UINT32 variable's value is an INT64, so it takes 8 too — `WriteNum(u, Num_UINT32)` writes 4 (explained) | unit |
| SIZEOF, VARPTR, MEMCPY, a TYPE passed to a DLL | an INT64 field encoded as a LONG (4 bytes: cut) | 8 bytes (UINT32 4), read back as the type | unit (memory), everywhere |
| PEEK / POKE | bytes | bytes (unchanged) | — |
| GET / PUT records, MKI$ / CVI-style | — | not in RapidQ (KEYWORD.LST, RC.EXE) nor RapidR: a program's binary numbers go through streams (the rows above) | — |
| DLL call: an INT64 / UINT64 result | a classic number (PRINT -2147483648 past 32 bits) | the 64-bit value (x64, ARM64: the register; x86: EDX:EAX) | Windows VM (§6) |
| DLL call: an INT64 by value on 32-bit Windows | **4 bytes pushed**: the stack the DLL reads was wrong | 8 bytes, low word first | Windows 7 build (§6) |
| DLL call: UINT32 parameters and results | not a type | a number (4 bytes), the result unsigned | unit (dll) |

## 5. Data, components, tools

| Surface | Before | Now | Test |
|---|---|---|---|
| RJson (desktop) | `Get` gave a classic number (past 32 bits shown -2147483648), a UINT64 above 2^63 a rounded DOUBLE; `Set` of a UINT64 above 2^63 rounded | exact both ways (`rapidr_value::json_doc`, one implementation for every runtime) | everywhere (`JSON`), unit `json_doc` |
| RJson (web) | the browser's JSON: every number a double (rounded past 2^53) | the desktop's implementation (serde_json, exact) | web conformance (everywhere) |
| RJavaScript (web) | a 64-bit value as a rounded double | a Number up to 2^53, a BigInt beyond; a BigInt coming back is a 64-bit value | — (web; covered by the shared conversion) |
| SQLite parameters | a UINT64 went as text (an INTEGER column then a rounded REAL) | an INT64 an INTEGER; a UINT64 above 9223372036854775807 refused: "doesn't fit in SQLite's whole numbers … pass it as text, STR$(n)" | unit (sqlite), manual |
| MySQL parameters | text | INT64 an integer, UINT64 `BIGINT UNSIGNED` | — (needs a server; the binding is one line) |
| Database results (`Row`, `Field`) | text | text, read into a 64-bit variable exactly | everywhere (the same store as `file`) |
| RDataFrame / RNum columns | DOUBLE columns | DOUBLE (RNum's numbers are f64 by design: exact to 2^53); part B (INT128 / BIGINT) brings exact integer columns — **open, part B** | — |
| Component properties | a 64-bit value stored as given; read back the same | the same, and shown in full (`Caption = u`) | — |
| Event arguments | made by the runtimes, 32-bit sizes | unchanged | — |
| The designer / inspector | an int property typed past 32 bits became a classic number | a 64-bit value kept; text past 32 bits an INT64 | unit (inspector `normalize`) |
| QREGISTRY ReadInteger / WriteInteger | REG_DWORD, 32 bits (RapidQ's); a 64-bit value was cut silently | REG_DWORD as before (a UINT32 its 32 bits); a 64-bit value beyond them stops the program: "WriteInteger keeps 32 bits (a REG_DWORD): … WriteString(name, STR$(n)) keeps it whole" (Spanish too) | probe run 2026-11-05 (the message, the value refused); WriteString / ReadString keep every digit |

## 6. Runtimes, the debugger, AI

| Surface | Before | Now | Test |
|---|---|---|---|
| Interpreter (JIT off) | static rewrites into host calls (`__cmp64` …) | the values' own kinds through the generic operations | the whole conformance suite (vm) |
| JIT (`RAPIDR_JIT=on` / `always`) | compiled INT64 as a classic integer (its `__cmp64` & co. were host calls, so a 64-bit loop mostly left) | `K::I64` / `K::U64` compiled; the same answers | jit, conformance (vm-jit 676/676); a 10 M-turn 64-bit loop 3172 ms (development build) → 30 ms |
| Native builds | the same rewrites | the same `Value` and functions as the interpreter | conformance (codegen) |
| Web | the same rewrites in wasm | the same VM and `rapidr-value` | web conformance, web input |
| 2.122.1 bytecode | — | its `__str64` & co. still answer (they read the new kinds too) | unit `wide::old_rewrites_still_answer` |
| A 2.123 module on a 2.122 runtime | — | refused with "needs RapidR Runtime 2.123.0" (a 64-bit constant, a UINT32 conversion or a new builtin: `Module::min_runtime`) | unit (bytecode) |
| Debugger: Variables, Watch, data tips | a UINT64 showed negative, an INT64 in a VARIANT cut | every digit; the kind says INT64 / UINT64 | the real walk (Studio, §7) |
| Debugger: Set Value | parsed as an i64 (a UINT64 above 2^63 gave 0) | read exactly; a value that doesn't fit is refused with the type's range | unit (session `fit_value`) |
| Immediate pane | the same VM as the program | the same | — |
| AI tools (RAI, MCP) reading a program's values | the debugger's text | the debugger's text, every digit | — (same path as Variables) |
| RMcpServer tool arguments | JSON text | JSON text, read exactly by RJson | everywhere (`JSON`) |
| Windows 7 (32-bit x86) | i686 build | i686 build with the 8-byte INT64 DLL argument (§4); the build compiles | the Windows 7 build (`tools/win7/build.sh`) |

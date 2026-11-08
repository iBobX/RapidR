# RR-SANE, part 1: what RapidR does only because RC.EXE does

Audit only (2026-11-06). No code was changed. The question: which behaviours of
the classic compiler (RC.EXE) does RapidR reproduce on purpose that surprise a
person and that nothing sensible depends on, so that a `.rr` program could do
the sensible thing while `.bas` / `.rqw` / `.rqb` programs stay exactly as
RC.EXE runs them?

Words used below:

- **CHANGE** = in `.rr` do the sensible thing; classic programs unchanged.
- **KEEP** = leave it the same in both.
- **ASK** = a real trade-off; a decision is needed (the lean is given).
- **Size**: S = a day or less (a few lines in one shared place, the `.rr`/`.bas`
  twin tests, the manual line); M = two or three days (several places or a new
  helper); L = a week or more.
- **Risk** = the chance that a working `.rr` program changes its output.
- **Importer**: what a conversion of a `.bas` to a `.rr` should do for a program
  that used the quirk. **R** = rewrite the code so it still does exactly what it
  did; **W** = leave the code, list the place in the report; **C** = the file
  stays classic (not renamed to `.rr`).

How it was checked: every claim about what happens today was run with the built
`target/release/rapidr` (built this morning, after the last language change) on
the same program saved as `.bas` and as `.rr`; the sites are read from the
source at HEAD (4d072aa6). RC.EXE itself was not re-run: its answers are the
record in `docs/rapidq-ground-truth.md`.

## The table

Sorted by what to do, then by how much it surprises people.

| # | Quirk (what RC.EXE does that RapidR copies) | Recommendation | Size | Risk |
|---|---|---|---|---|
| 1 | `IF NOT FILEEXISTS(f)` / `IF NOT Check.Checked`: NOT of a 1 is -2, which is true | CHANGE | S | low |
| 2 | `IF Check.Checked = True` is never true in a `.rr` (True is -1, Checked is 1) | CHANGE (cheap form) | S | low |
| 3 | Array subscripts are not checked: `a(5)` on `DIM a(3)` works, `Grid(i, NumX + 1)` is the next row | CHANGE | M | medium |
| 4 | `INT(-2.5)` is -2 (INT truncates, the manual says floor) | CHANGE | S | medium |
| 5 | `ROUND` / `CINT` / `CLNG` are `INT(x + 0.5)` truncating: `ROUND(-2.7)` is -2; results are 32-bit | CHANGE | S | medium |
| 6 | `FORMAT$`: no zero flag (`%02d` of 5 is ` 5`), `%s` ignores numbers, `% d` / `%+d` stop the program | CHANGE | M | low |
| 7 | `STR$` keeps 9 digits: `STR$(1234567890)` is `1.23456789E9` | CHANGE | S | low |
| 8 | `PRINT` of a whole number beyond 32 bits, of an infinity or NaN shows -2147483648 | CHANGE | S | low |
| 9 | `PRINT a, b` glues the items (`PRINT 1, 2` prints `12`) | CHANGE | S | low |
| 10 | `&` is `+` (`5 & 3` is 8) and `&&` is not AND (`IF x = 3 && y = 4` is never true) | CHANGE | S | low |
| 11 | One-line IF: the PRINT before ELSE drops its new line (approved) | CHANGE | S | low |
| 12 | One-line IF: a spare second ELSE, an outer ELSE after a nested IF; block IF: a second ELSE whose code never runs | CHANGE | S | low |
| 13 | `""` inside a string is not a quote: `"a""b"` is `a` | CHANGE | S | low |
| 14 | A string never closed is closed silently at the end of the line | CHANGE | S | low |
| 15 | Operands side by side and dropped operators: `2(3)` is 2, `10 - 2 3` is 10, `IF k > - THEN` compiles | CHANGE | S-M | low |
| 16 | A `FUNCTION F AS INTEGER` returns `2.7` unconverted | CHANGE | S | medium |
| 17 | Undeclared variables: order-dependent scope, static inside a SUB, a typo makes a new variable | ASK (lean CHANGE) | M | medium |
| 18 | The program ends when its main code ends: `Form.Show` without `ShowModal` flashes and closes | CHANGE | M | low |
| 19 | `VAL("&HFF")` is 0 and `READ` of `DATA &H10` into a number is 0 | CHANGE | S | low |
| 20 | `Obj.Method = a, b, c` calls the method; `L.AddItems = "x"` adds an empty line | CHANGE | S | low |
| 21 | `FOR n = TO 29` (no start) compiles and counts from 0 | CHANGE | S | low |
| 22 | A stray `END TYPE` / `END STRUCT` is `END`: the program stops there | CHANGE | S | low |
| 23 | `&HFFG` is 255, `&H1?` is 16, `&HH1` is 1 (any letter or `?` after `&H` is read) | CHANGE | S | low |
| 24 | `DIM x, y AS INTEGER` types only `y` (RC.EXE itself refuses the line) | CHANGE (both modes) | S | low |
| 25 | NaN compares equal to everything (`=`, `<`, `<=` true) | CHANGE | S | low |
| 26 | `QFileStream.ReadStr(n)` returns n + 1 characters (a trailing space) | CHANGE | S | low |
| 27 | `INSTR(-5, "abc", "b")` is -4; a real start gives a real place (`3.5`) | CHANGE (low value) | S | low |
| 28 | A literal beyond 32 bits is 64-bit only in programs that mention INT64 / UINT64 / UINT32 | CHANGE (always, in `.rr`) | S | low |
| 29 | `&O17` / `&B101` are numbers in a classic program too (RC.EXE reads variables O17 / B101) | CHANGE (in classic) | S | low |
| 30 | `DWORD` is signed and `d = 3000000000` holds -2147483648 | ASK (lean CHANGE) | S | low |
| 31 | An integer store beyond 32 bits becomes -2147483648; a BYVAL INTEGER parameter rounds where a store truncates | ASK (lean KEEP) | S | medium |
| 32 | `/` by zero gives an infinity silently (`\` and MOD stop with an error) | ASK (lean KEEP) | S | medium |
| 33 | Unary minus binds tighter than `^`: `-2 ^ 2` is 4; `2 ^ 3 ^ 2` is 64 | ASK (lean KEEP) | S | high |
| 34 | Fractions print with exactly 9 decimals (`3.500000000`), with emulated x87 digits past the 15th | ASK (lean KEEP the shape) | S | high |
| 35 | An integer store truncates (`i = 2.7` holds 2) | KEEP | - | - |
| 36 | Bit operators, `\` and MOD work on 32 bits and round their operands; a shift count uses 5 bits | KEEP | - | - |
| 37 | `&H80000000` to `&HFFFFFFFF` are negative numbers | KEEP | - | - |
| 38 | `HEX$` has 8 digits; `BIN$` / `CONVBASE$` of a negative number give its 32 bits | KEEP | - | - |
| 39 | `TIMER` is seconds since midnight | KEEP | - | - |
| 40 | `"jello" - "l"` removes characters | KEEP | - | - |
| 41 | `$DEFINE` matches in any case | KEEP | - | - |
| 42 | A FUNCTION's own name inside it is a call; `RESULT` is the value | KEEP | - | - |
| 43 | A TYPE's PROPERTY SET is not called from the TYPE's own code | KEEP | - | - |
| 44 | `DIM m` with no type is a DOUBLE; `STRING * n` pads; `DIM a(3)` has 4 elements; parameters are BYVAL | KEEP | - | - |
| 45 | `FILEEXISTS` / `DIR$` wildcards; `Clipboard.GetAsText(n)` gives n - 1 characters | KEEP | - | - |
| 46 | Name rules (a type's name, a name twice, STEP / TO / THEN / AND as SUB names) | KEEP | - | - |
| 47 | Harmless forms the examples need (`...` in a DECLARE, DECLARE parameter names, `$INCLUDE "file` with no closing quote) | KEEP | - | - |

Three decisions (A, B, C) come first, after the table: A (one mode per program or
per file) decides how hard the "error" rows 12-15, 19, 21-23 are; B (one truth
value) is rows 1-2; C is how the importer uses all this.

## Where we start

**What already differs in a `.rr` today.** A `.rr` program is marked by the
preprocessor with a last line `$OPTION RAPIDR` (`crates/rapidr-preprocessor/src/lib.rs:267`,
added at `:668`). Three places read it:

- the parser (`Parser.rapidr`, `crates/rapidr-parser/src/lib.rs:297`): blocks end
  in their own file, a WITH or CREATE left open is an error, the messages are
  RapidR's own where a classic program gets RC.EXE's words (`:400-460`);
- the AST passes (`rapidr_ast::builtin_constants::is_rapidr_program`): RAPIDQ.INC's
  constants are built in, `True` is -1, `ctText` and friends exist;
- the runtime (`rapidr_value::rapidr_program()`, `crates/rapidr-value/src/lib.rs:1454`,
  set by the parser's first statement `__rapidr_program`; the VM, native builds
  and the web all run it): so far only the mouse wheel on scroll bars.

Also split by file name already: the console code page (a plain-ASCII `.bas`
prints CHR$ codes in code page 437, a `.rr` prints text as it is,
`crates/rapidr-preprocessor/src/lib.rs:520`) and the source encoding (a `.rr`
must be UTF-8, `crates/rapidr-langsvc/src/diagnostics.rs:112`). The language
service warns only in a classic file about a RAPIDQ.INC constant used without
the include (`hints.rs`).

**The importer today never writes a `.rr`.** `rapidr import-rapidq` keeps every
file's extension (`.bas` stays `.bas`), changes component names only (the tokens the
parser read as type names) and proves the copy compiles to the same bytecode
as the original (`crates/rapidr-import/src/lib.rs`, `docs/manual/importing-rapidq.md`).
So none of the changes below touches anybody's imported code until the importer
starts writing `.rr`. When it does, every row says what it should do (R / W / C
above). The byte-for-byte proof cannot hold for a `.rr` copy; the report's list
of rewrites and warnings takes its place.

**How a change is built, so all four runtimes agree.** Everything below is in
shared code, so the interpreter / VM, the JIT, native builds and the web follow
together. Two places need care: arrays exist twice (`BasicArray` and the typed
arrays of native builds, `typed_array.rs`), and PRINT is reached from three
hosts (the VM, `rapidr-runtime-core`, `rapidr-runtime-web`), each of which must
see the flag. Three kinds:

1. *Syntax rows* (9-12, 15, 20-22, 24, and 13, 14, 23 in the lexer; row 10's `&&`
   error): the parser has
   `self.rapidr`. The lexer does not know the mode yet (it is handed the source
   with `$OPTION RAPIDR` at its end), so rows 13, 14 and 23 need a flag passed in;
   it already receives the file path.
2. *Library-call and AST rows* (4, 5, 6, 7, 19, 27, 16, 17, 28, 30): an AST pass
   renames the call, or inserts the conversion, when the program is a `.rr` (as
   `builtin_constants` and `rapidr_ast::numeric` already do). No run-time switch;
   the VM's table of direct maths calls (`interpreter/rapidr-vm/src/fast.rs`, named
   in `fast_tests.rs:612-640`) gets at most two more entries (INT, ROUND), and the
   JIT calls the same table.
3. *Value-model rows* (3, 8, 10's `&`, 25, 26): in `rapidr_value` behind the existing
   `rapidr_program()` flag (program-wide), because a value or an object does not
   know which file made it. Row 18 is in the three hosts.

Every CHANGE also needs: a `.rr` conformance case (the suite already has 23 `.rr`
cases) beside the unchanged `.bas` case, run on all three targets; the manual
line; an example under `examples/`; the Help / AI facet (the "manual + example
for every change" rule). The existing `rapidq_*` `.bas` cases keep proving the
classic side.

### Decision A: one mode per program, or per file?

Today the mode is the main file's extension and it covers every `$INCLUDE`.
Consequences:

- A `.rr` that includes a classic library (`RAPIDQ2.INC`, a user's own `.inc`
  from RapidQ days) compiles that library by `.rr` rules. Rows 12-15, 19 and 21-23
  turn forms RC.EXE accepts into errors, and RapidQ's own include libraries use
  exactly those forms: `CommCtrl.inc` writes `&HFFFF0000???` (row 23);
  `Qcdaudio.inc` ends an argument with a dangling `+,` (row 15); `RAPIDQ2.INC`
  makes `CONST 1 = 1` out of `$DEFINE TRUE 1` + `CONST True = 1` (parser
  `:913-931`); `QLed.inc` closes a PROPERTY SET with `END SUB`. Examples do too:
  `Window Sizer` has the second ELSE (row 12), `qdownload` ends with a stray
  `END STRUCT` (row 22), `joystick` has `IF Buttons > -` (row 15), `StutterBox` has
  `FOR N = TO 29` (row 21).
- A `.bas` that includes a `.rr` compiles RapidR's code by classic rules.
- RapidR's own Studio program is `.inc` files included from a `.rr` (`ide/*.inc`),
  so a file extension alone cannot say `.inc` = classic.

**Lean: per file**, with this rule: a file's mode is its own extension (`.rr` =
RapidR; `.bas` `.rqw` `.rqb` `.rq` = classic); an `.inc` follows the file that
includes it; any file found through an include folder outside the project is
classic; `$OPTION RAPIDR` / `$OPTION CLASSIC` at the top overrides. The cost is
that kind-3 rows cannot use a program-wide flag: the DIM that makes an array and
the PRINT that shows a number would be lowered to a variant by their file's mode
(S each). If per-file is refused, then every "error" row needs a check that
RapidQ's own include files and the 428 example programs still compile as the
includes of a `.rr` before it lands.

### Decision B: one truth value

A comparison gives -1; `True` in a `.rr` is -1; but a component's Boolean
(`Checked`, `Enabled`, 375 properties in the registry), `FILEEXISTS` and
`DIREXISTS` give 1, and NOT is bitwise. That makes rows 1 and 2. RapidR's own
programs already live with it: they write `running = -1`, not `= 1`
(`examples/graphics/pixels_fast.rr:269`), and the ledger lists both mistakes as
repeated (`docs/rai-mistake-ledger.md` rows 5 and 6). Options:

1. *(recommended, cheap, additive)* keep every value as it is, and make the two
   traps impossible by rule: in a `.rr`, `x = True` / `x <> True` / `x = False`
   compare truth (non-zero or zero), and `NOT` in a condition (IF, ELSEIF, WHILE,
   UNTIL, IIF) is "is zero" when its operand is not a comparison. Done in the
   shared AST pass, no new value kind.
2. every yes/no reads -1 in a `.rr`: breaks the very common `IF Check.Checked = 1`.
3. a real Boolean type (`DIM x AS BOOLEAN` is `Unknown data type BOOLEAN` today and
   is in the ledger as a repeated mistake): the clean answer, a larger project;
   `Value::Boolean` already exists inside the interpreter.

### Decision C: the importer's second step

Keep `import-rapidq` as it is (classic copy, proven identical). Add an explicit
second step beside `upgrade-names`, "bring this program to the RapidR language",
that writes `.rr` and applies R / W / C per row. Its report lists every rewrite
and warning with file:line. Nothing about an old program changes unless that
step is run.

## Details

Each row: what RC.EXE does and what a person expects; where it lives; runtimes;
what a `.rr` does today; the recommendation, size and risk; the importer.

### Truth values (rows 1, 2)

**1. NOT of a 1.** `NOT 1` is -2. `FILEEXISTS`, `DIREXISTS` and every component
Boolean give 1, so `IF NOT FILEEXISTS(f) THEN` takes its THEN for an existing
file as well, and `Check.Checked = NOT Check.Checked` never unchecks (Checked
keeps 0 / 1, so -2 reads 1). A person expects "not true".
Probe (`.bas` and `.rr` alike): `IF NOT FILEEXISTS("/etc/hosts") THEN` runs its THEN.
Where: parser `parse_not` `crates/rapidr-parser/src/lib.rs:3319`; value
`property_read` `crates/rapidr-value/src/lib.rs:674`; the registry says "True (1)"
for the two builtins (`crates/rapidr-lang/data/builtins.toml:334,388`). All
runtimes (AST). Already differs in `.rr`: no (the editor warns about `NOT FILEEXISTS`
only: `crates/rapidr-langsvc/src/hints.rs:35`, and the AI card has a rule).
Probe: `C.Checked = NOT C.Checked` on a checked box leaves it checked.
Recommendation: CHANGE by Decision B option 1: in a condition, `NOT x` where `x`
is not a comparison means `x = 0`. Nothing sensible reads a bitwise NOT as a
condition (it is true for every value but -1). `NOT` as a value (`running = NOT
running`) stays bitwise. S, low.
Importer: W (the fix changes only code whose old answer was wrong; list the places).

**2. `Checked = True`.** In a `.bas` with RAPIDQ.INC `True` is 1 and
`IF Check.Checked = True` works; in a `.rr` `True` is -1 and it is never true
(probe: prints `Checked = True is FALSE`; in a `.bas` without the include `True` is
an undeclared 0, so it fails there too). The manual already tells people not
to write it (`docs/manual/language.md:123`); a language hint exists
(`crates/rapidr-langsvc/src/foreign.rs:117`). Where: `crates/rapidr-ast/src/builtin_constants.rs`
(`vtTrue`). Recommendation: CHANGE by Decision B option 1 (a comparison against
the literal `True` / `False` compares truth). S, low. Importer: nothing needed
(the comparison now works).

### Arrays (row 3)

**3. Unchecked subscripts.** RC.EXE does no bounds check: `a(5) = 9` on
`DIM a(3)` works, `a(-2)` works, and with `DIM g(2, 2)` the index `g(0, 3)` is
`g(1, 0)` (RapidQ's own `Grid(i, NumX + 1)` relies on the next row). RapidR keeps
this memory-safely (stores outside the array are kept beside it and read back).
A person expects "Subscript out of range" on the line of the mistake; the usual
off-by-one in a FOR loop becomes a silent wrong sum (probe: `FOR i = 0 TO 4`
over `DIM a(3)` sums to 10, no message).
Where: `BasicArray::offset/get/set` `crates/rapidr-value/src/lib.rs:277-380`
and a second copy in native builds' typed arrays `crates/rapidr-value/src/typed_array.rs:98-175`
(the tests compare the two). The JIT leaves compiled code on an out-of-range access
and the interpreter's `BasicArray` answers (`interpreter/rapidr-jit/src/compile.rs:1091`).
Both copies exist in all runtimes already, so one flag in both. Already differs
in `.rr`: no (the message "Subscript out of range: index 7 is outside 0 TO 3" is
built by `offset` for a place outside the whole array and then swallowed by `get`
and `set`; the text and its translation exist). Recommendation: CHANGE: in a `.rr`, any subscript
outside its own bounds is a run-time error with the line; the per-dimension
check is the new part. M, medium: a `.rr` program that wrote past the end by
accident stops (that is the point); RapidR's own 120 examples and Studio should
be run once to see none relied on it. Speed: for a one-dimension array the
whole-array check is already the per-dimension check, so nothing is added; a
multi-dimension array adds one compare per extra index, and the "store beside
the array" path is no longer needed in `.rr`.
Importer: W (can't be found by reading; a one-line note in the report: "arrays
are checked"); C for a program known to need it (a `READ D(N)` filling past the
end, like RapidQ's own `GMDrum`).

### Numbers (rows 4, 5, 16, 28, 30-34, 35-38)

**4. INT.** `INT(-2.5)` is -2, `INT(-0.5)` is 0: RC.EXE truncates, its manual says
"largest integer less than or equal". A person expects -3. Where:
`rp_int` `crates/rapidr-value/src/builtins.rs:279-289`; in the VM's direct maths
table (`interpreter/rapidr-vm/src/fast.rs`). Same as FIX in a classic program.
`.rr` today: same. Recommendation: CHANGE: in a `.rr`, INT floors, FIX stays
truncation. S, medium (72 uses of `INT(` in RapidR's own programs; nearly all are
on positive values: `INT(RND * n)`, `INT(t / 60)`. The old sample IDE,
`examples/ide.rr:238`, snaps `INT((V + 4) / 8) * 8`, where a negative V now rounds
the other way: such centring and snapping code needs a look; Studio itself, in
`ide/`, has 6 uses, all on timings and positive sums).
Importer: R, exact: `INT(` becomes `FIX(`.

**5. ROUND / CINT / CLNG / CEIL / FLOOR.** They are `INT(x + 0.5)` truncating:
`ROUND(2.5)` 3 but `ROUND(-2.7)` is -2, `ROUND(-2.2)` is -1, `ROUND(-3.99)` is -3;
and the result is a 32-bit integer (`ROUND(1E10)` is -2147483648; CEIL / FLOOR
too). A person expects the nearest whole number. Where: `crates/rapidr-value/src/builtins.rs:290-310,410,484-495`.
Recommendation: CHANGE: nearest, ties away from zero (so every non-negative result
is exactly what a classic program gets; only negatives and out-of-range values
change, and those were wrong), no 32-bit cut. S, medium.
Importer: W. (An exact rewrite exists, `FIX(x + 0.5)`, but it keeps the wrong
answers; the cases that differ are only the ones where the old answer was wrong.
Part 2 can choose.)

**16. FUNCTION result not converted.** `FUNCTION F AS INTEGER : F = 2.7` returns
2.7 (a store `i = 2.7` gives 2). A person expects the declared type to hold.
Where: `crates/rapidr-ast/src/numeric.rs:14` (not converted on purpose),
`crates/rapidr-value/src/numeric.rs:14-15`. Recommendation: CHANGE: convert the
result as a store converts. S, medium (loosely typed code that returns a
fraction through an INTEGER function now truncates). Importer: W.

**28. 64-bit literals by use.** A whole-number literal beyond 32 bits is an INT64
only in a program that mentions INT64 / UINT64 / UINT32; elsewhere it is read the
classic way (`PRINT 2147483648` shows -2147483648). Adding `DIM x AS INT64` on
another line changes what an unrelated literal prints. Where:
`docs/big-numbers-design.md` §3, `crates/rapidr-ast/src/wide_literals.rs`.
Recommendation: CHANGE: always 64-bit in a `.rr` (the detector already exists;
the flag turns it on). S, low. Importer: nothing.

**30. DWORD signed.** `d = -1` prints -1; `d = 3000000000` holds -2147483648
(not even a wrap). Every other language's DWORD is unsigned. RapidR already
has `UINT32` (wraps to 32 bits unsigned, `docs/big-numbers-design.md` §4).
Where: `crates/rapidr-value/src/numeric.rs:13`. Recommendation: ASK, lean CHANGE:
in a `.rr`, `DWORD` is `UINT32`. DLL calls pass the same 32 bits. S, low.
Importer: W where a DWORD is compared with a negative number.

**31. Integer stores.** A store beyond 32 bits (or NaN) becomes -2147483648
(`i = 1E10`), the x87's "integer indefinite", neither a wrap nor an error; and a
BYVAL INTEGER parameter rounds half to even where a store truncates (`P 2.7` gets
3, `i = 2.7` gets 2; `crates/rapidr-ast/src/numeric.rs:300`). ASK, lean KEEP
the store (overflow semantics programs may lean on; you asked for these to be
left alone) and CHANGE the parameter to truncate like a store (S, low): one rule
for "a fraction going into an integer slot". If a run-time "Overflow" error is
wanted for the out-of-range store instead, it is S but changes working
(if wrong) programs: separate decision.

**32. `/` by zero.** `7 / 0` is an infinity and goes on, silently (and PRINT
shows -2147483648, row 8). A person expects an error like `\` and MOD give.
ASK, lean KEEP: BASIC's AND does not short-circuit, so the usual guard
`IF d <> 0 AND n / d > 1 THEN` evaluates the division anyway and works in a
classic program only because the infinity is harmless; an error here would break
that idiom in `.rr`. Row 8 (print it as INF) fixes the visible part.

**33. `^` and unary minus.** `-2 ^ 2` is 4 (RapidQ's manual puts negation and
`^` at the same level, left to right: appendix C, kept locally in
`.reference/phatcode/operators.html`),
`2 ^ 3 ^ 2` is 64. Many languages read -4 for the first. It is documented and
numeric results get copied between files. ASK, lean KEEP (the one place where
a "fix" silently changes a number).

**34. Fractions print with 9 decimals.** `3.500000000`, `0.333333333`;
`1234567890.1` prints `1234567890.099999840` (the x87 conversion's 18 digits,
emulated on purpose in `crates/rapidr-value/src/format.rs:471-520`). This is
the number-formatting convention you asked me not to touch; two sub-points to
decide: (a) keep the 9-decimal shape, but print the correctly rounded digits
(`1234567890.100000000`), S, low-medium, since only values past about 1E7 with a
fraction differ; (b) the whole shape (`3.5`, S, high: every printed fraction in
every `.rr` example and test changes). Lean: KEEP the shape, do (a) if wanted.

**35-38.** Kept because they are conventions programs rely on: integer stores
truncate (`i = 2.7` is 2, the same rule as C); `AND OR XOR NOT SHL SHR \ MOD`
work on 32-bit integers, round their operands and use 5 bits of a shift count
(`crates/rapidr-value/src/lib.rs:758-880`; a program that wants 64 bits uses
INT64, where these operate on 64); `&H80000000` to `&HFFFFFFFF` are negative
(the system colours `clBtnFace = &H8000000F` are built on it); `HEX$` has 8 digits
(documented), `BIN$` 32 bits.

### Text and printing (rows 6-10, 19, 26, 27)

**6. FORMAT$.** RC.EXE's table: `%02d` of 5 is ` 5` (a `0` before a width is only
a width digit: `%05d` of 42 is `   42`); `%s` shows a string and nothing for a
number; `%d` of a string is 0; `%+d`, `% d`, `%#x`, `%'d` and a lone `%`
(`"50% off"`) stop the program with an exception (`Format '% ' invalid or
incompatible with argument.`; probe: `FORMAT$("50% off")` ends the program with a
run-time error in a `.rr` as well). A person expects zero padding, `%s` of 42 to
show `42`, `+`, space and `#` flags to work. Where: `crates/rapidr-value/src/format.rs:33-46`
and its table; `rp_format` `crates/rapidr-value/src/builtins.rs:686`. RapidR's
own programs never write `%0` (none of 84 FORMAT$ calls in `examples/` and `ide/`
has a zero flag).
Recommendation: CHANGE: C-style `printf` rules in a `.rr`. M (the spec parser
has a branch per RC.EXE rule), low.
Importer: R, exact, when the format is a literal: drop a `0` flag (`%02d` becomes
`%2d`). But that preserves output the author almost surely did not want, so W
may be better for this one: list every FORMAT$ with a `0` / `+` / space / `#`
flag or a `%s` and let the author choose. Part 2 picks.

**7. STR$.** `FloatToStrF(x, ffGeneral, 9, 0)`: 9 significant digits, scientific
when shorter: `STR$(1234567890)` is `1.23456789E9`, `STR$(1234567.891)` is
`1234567.89`, so an id or phone number built with STR$ breaks past 9 digits.
Meanwhile RapidR's implicit number-to-text conversion (`"x" + n`) uses 15
digits (`float_to_str`, `format.rs`): `LEN(1/3)` is 17 and `LEN(STR$(1/3))` 11.
Where: `str_number` `crates/rapidr-value/src/format.rs:538`. Recommendation:
CHANGE: STR$ gives what the implicit conversion gives (up to 15 digits, whole
numbers in full). S, low. Importer: W.

**8. PRINT of a big number, Inf, NaN.** `PRINT 1E10`, `PRINT 7 / 0` and
`PRINT 2147483648` show -2147483648. A person expects the number, or `INF`.
Where: `print_double` and `print_text` `crates/rapidr-value/src/format.rs:471,523`
(`int32`). All runtimes call them. Already differs when the program uses INT64
(it prints whole). Recommendation: CHANGE: in a `.rr` a whole number prints in
full (up to 2^53 exact in a DOUBLE) and an infinity / NaN print `INF` / `NAN` as
STR$ does. S, low. Importer: nothing (the old text was wrong).

**9. PRINT's comma.** `PRINT 1, 2` prints `12`, `PRINT "x =", x` prints `x =5`
(the comma is the semicolon, numbers have no leading space; documented in the
classic manual). Every BASIC that has a comma gives it a meaning (columns).
The AST and both back ends already carry print zones (`PrintStatement.zones`,
`crates/rapidr-ast/src/lib.rs:234`, `interpreter/rapidr-bcgen/src/lib.rs:1543`,
`crates/rapidr-codegen-rust/src/lib.rs:1388`); the parser
sets them false (`parse_print`, `crates/rapidr-parser/src/lib.rs:1452`).
Recommendation: CHANGE: in a `.rr` the comma moves to the next 14-column zone
(set `zones[i]` true). S, low. Importer: R, exact: a comma between PRINT items
becomes `;`.

**10. `&` and `&&`.** `&` is `+`: `5 & 3` is 8 and `"5" & "3"` is 53; `&&` is the
same operator (the second `&` is dropped as in `&x`), so `IF x = 3 && x = 3`
reads `x = (3 + x) = 3` and is never true (probe: `1 && 2` is 3). People coming
from other languages hit this constantly (ledger row 10). Where: `Value::concat`
`crates/rapidr-value/src/lib.rs:817`, parser `parse_unary` `:3489`. Recommendation:
CHANGE: in a `.rr`, `&` always joins text (`5 & 3` is `53`), `&&` is an error with
the fix "use AND" (the `||` case is an error already). S, low. Importer: R,
exact and type-free: every binary `&` becomes `+`, `&&` becomes `+`.

**19. VAL and DATA with `&H`.** `VAL("&HFF")` is 0, a `DATA &H10` read into a number
is 0 (kept as the text `&H10` in a STRING), while `PRINT &H10` is 16. Nobody sensible
relies on a 0 here. Where: `rp_val` `crates/rapidr-value/src/builtins.rs:225`;
`data_items` `crates/rapidr-ast/src/lib.rs:3855`. Recommendation: CHANGE: `&H`
(and `&O`, `&B`) are read by VAL and in DATA in a `.rr`. S, low. Importer: W (a
program that read 0 from such a DATA now reads the number).

**26. ReadStr.** `QFileStream.ReadStr(3)` of `abcdefgh` returns `abc` plus a space
(4 characters) and moves 3; `ReadBinStr(3)` and a memory stream's ReadStr give 3
(probe: `[abc ] len=4 pos=3`). `s = f.ReadStr(f.Size)` has a spurious trailing space.
Where: `read_str` `crates/rapidr-value/src/objects/memstream.rs:208-217` (also
`Read(S$)`). One shared object model for all runtimes. Recommendation: CHANGE:
n characters in a `.rr`. S, low. Importer: W (a program that trims or counts the
extra space would break: list each `ReadStr` / `Read` on a file stream).

**27. INSTR starts.** `INSTR(-5, "abc", "b")` is -4 (the place is counted from the
given start), `INSTR(2.5, "abcabc", "c")` is `3.5`; RINSTR's start works on
`LEFT$`. Where: `crates/rapidr-value/src/strings.rs:67-130`. Recommendation:
CHANGE (low value): a start below 1 is 1, a real start is rounded; the result
is always a whole place. S, low. Importer: W.

### The one-line IF and other syntax (rows 11-15, 20-24)

**11. PRINT before ELSE** (approved). `IF 1 THEN PRINT "a" ELSE PRINT "b"` then
`PRINT "c"` prints `ac` (probe: the `a` has no new line). Where: parser
`print_ends_without_newline` `lib.rs:239`, called at `:1979` and `:2002`. In `.rr`:
the PRINT ends its line. S, low. Importer: R, exact: write `PRINT "a";`
(semicolon) in the THEN part, which keeps the old output.

**12. ELSE pairing.** (a) `IF 0 THEN IF 1 THEN a ELSE b ELSE c` runs nothing: the
outer `ELSE` is taken as the inner IF's spare, so the outer IF has no ELSE
(`lib.rs:1957-2024`, `LineElse`; probe: nothing printed); (b) a second `ELSE` on a
one-line IF is accepted and the statement after it never runs
(`IF 0 THEN a ELSE b ELSE c` runs b); (c) a second `ELSE` in a block IF compiles
and its body never runs (`:2069-2090`; probe: prints `a`, `end`, the "c" branch is
dead code). A person expects the usual pairing: an ELSE belongs to the nearest
IF that has none yet, so in (a) the last ELSE is the outer IF's and `c` runs; and
a second ELSE with no IF left for it to be an error. Recommendation: CHANGE: the
usual pairing for (a); (b) and (c) are errors ("a second ELSE: it has no IF").
S, low. Importer: R, exact for (b) and (c): delete the second ELSE and its dead
statements (they never ran); W for (a) (the old pairing is rarely what was meant;
list it). The conformance cases `if_else_forms`, `if_third_else`, `rapidq_two_else`
and `rapidq_if_print_else` stay as the `.bas` side.

**13. `""` in a string.** RC.EXE reads `"[:"":>"` as two strings side by side, worth
the first: `"a""b"` is `a`, `LEN` 1 (probe). Every other BASIC reads a quote.
Where: lexer `crates/rapidr-lexer/src/lib.rs:940` (it ends the string at the
first quote). Recommendation: CHANGE: `""` is a quote in a `.rr`. S, low. Importer:
R, exact: `"a""b"` becomes `"a"` (the juxtaposed rule: the value is the first string).
RapidQ's own programs write CHR$(34).

**14. Unterminated string.** `x$ = "error` (no closing quote) is a string to the end of
the line, silently; the conformance case calls it "jello style" leniency
(`rapidq_syntax`). Where: lexer `:957-962` (only the end of the *file* is an
error). Recommendation: CHANGE: an error at the opening quote, with the fix.
S, low. Importer: R, exact: add the missing `"` at the end of the line.

**15. Operands side by side, dropped operators.** RC.EXE reads on instead of
refusing: `2(3)` is 2, `A B OR C` is A, `-9(COS(x))` is 9, `10 - 2 3` is 10, the
RPN form `(4) (7) (*)` the manual describes, `IF k > - THEN` compiles
(probe: prints `weird`; `10 - 2 3` prints 10), `x = = 5`, `&x` / `#H1F` (a name),
a lone `_` as an operand 0 (`t _ (3 + 1)`), a trailing operator is dropped.
The dropped operands are still evaluated (a FUNCTION among them is called). A
missing `+` or `*` becomes a silently wrong number. Where: parser
`parse_expression` `lib.rs:2969`, `parse_stacked_expression` `:3022-3200`,
`parse_unary` `:3477-3500`; lexer `:251`. Recommendation: CHANGE: in a `.rr`
these are the syntax error they look like ("two values with no operator
between"; "the operator has nothing after it"). The documented postfix form goes
with it (nobody writes it). S-M, low. Importer: C if any occurs and the dropped
operand has side effects, else R (write the value RC.EXE keeps); W is the safe
default because the classic result was almost surely not meant.

**20. `Obj.Method = a, b, c`.** A method "assigned" is called, its first argument
`= a`: `B.FillRect = 10, 10, 20, 20, &HFF` fills; `L.AddItems = "x"` adds an empty
line (probe: `ItemCount` 1). Property assignments with commas are errors.
Where: parser `:1868-1885`, `rapidr_ast::method_equals`. Recommendation: CHANGE:
an error with the fix ("AddItems is a method: call it, `L.AddItems "x"`").
S, low. Importer: R when the form is `Obj.Method = a, b` (becomes
`Obj.Method a, b`, same arguments); W for the odd forms (`v.Bar 5 = 6, 7`).

**21. `FOR n = TO 29`.** No start value compiles and counts from 0 (probe: `0123`).
A typo accepted. Where: parser `:2114`. Recommendation: CHANGE: error. S, low.
Importer: R, exact: write `FOR n = 0 TO 29`.

**22. Stray END TYPE / END STRUCT.** With no TYPE open it is `END`: the program ends
there, even inside a SUB (probe: the line after is never reached). Where: parser
`:975-1010`. Recommendation: CHANGE: error "END TYPE without TYPE". S, low.
Importer: R, exact: write `END`.

**23. `&H` runs.** `&H` takes the whole letters-and-digits run and keeps the hex
digits: `&HFFG` is 255 (the G is dropped), `&hHE` 14, `&HH1` 1, `&H1?` 16 (`?` and
`@` are 0, from RapidQ's CommCtrl include). A typo in a constant gives a wrong
number silently (probe: `255 16 1`). Where: lexer `:978-1010`, parser `:3770`. Recommendation:
CHANGE: only hex digits follow `&H` (error otherwise). S, low. Importer: R,
exact: write the literal RC.EXE reads (`&HFFFF0000???` becomes `&HFFFF0000000`).

**24. `DIM x, y AS INTEGER`.** RC.EXE 2006 refuses the line; RapidR accepts it,
following the manual's reading, so only `y` is INTEGER and `x` a DOUBLE
(probe: `x = 2.5` stays 2.5, `y` becomes 2). Where: parser `:1290`, test
`:4645`. No classic program has this line, so changing it in both modes cannot
break one. Recommendation: CHANGE: both typed (as a person reads it). S, low.
Importer: nothing.

**25. NaN.** `d = 0 / 0` then `d = d` and `d < 1` are true (the x87 leaves "less"
and "equal"); `<>`, `>`, `>=` false. Where: `Value::rp_eq` etc.
`crates/rapidr-value/src/lib.rs:928`. Recommendation: CHANGE: every comparison is
false except `<>`. S, low (NaN is rare, and row 8 makes it visible). Importer: nothing.

**29. `&O` / `&B` in classic programs.** RapidR reads `&O17` and `&B101` as octal
and binary numbers in a classic program too (RapidR's addition); RC.EXE reads
`&` before a name as the name, so `PRINT &O17` prints the variable O17 (0
undeclared). The ground truth lists it as open and kept. Recommendation: CHANGE
in classic: only a `.rr` has these literals. S, low (matters only to a classic
program with a variable named like that). Importer: R, exact: a `.bas` with such
a name is left alone (it keeps meaning the variable).

### Declarations and scope (rows 17, 18)

**17. Undeclared variables.** RC.EXE's one-pass rules: a name the main program used
above a SUB is that global inside it; a name a SUB uses first is the SUB's own and
is *kept between calls* (a hidden STATIC); the main program's same name further
down is a different variable (probe: `Counter` prints n = 1, 2, 3, then main's
`n = 100` is another n). A typo makes a new variable silently (`PRINT totl` prints
0), and an undeclared name is a DOUBLE, so `name = "x"` fails with "expecting type
DOUBLE, but got STRING" (ledger row 4). Where: `rapidr_ast::implicit_scope`
(`crates/rapidr-ast/src/implicit_scope.rs`) inside `option_dim`
(`crates/rapidr-ast/src/lib.rs:898-960`); `$TYPECHECK ON` exists as an opt-in with
RC.EXE's words (`Undeclared identifier`, `Undefined symbol`). Measured (copies in
the scratch folder, nothing in the repository touched): with `$TYPECHECK ON` put
on top, 118 of the 120 example programs and RapidR Studio (`ide/studio.rr` and
its include files) compile with no error; the two that fail name three
variables (`CRLF$` in `gui/richtext.rr`, `NL$` and `Q$` in `studio/diff_views.rr`). Recommendation: ASK, lean CHANGE: a `.rr` requires declarations by
default (`$TYPECHECK OFF` to opt out), with friendly messages and "did you mean"
(`docs/errors-audit-2026-11.md` item 8). The scope rules then disappear for
`.rr`. M, medium (every user's `.rr` gets the error on first run).
Importer: R, exact: it already knows each name's scope; write a `DIM name AS
DOUBLE` at the top for each global and a `STATIC name AS DOUBLE` in the SUB for
each SUB-first name (type suffix kept). Needs no judgement.

**18. The program ends with its main code.** After `Form.Show` (no ShowModal, no
DOEVENTS loop) the main program ends and the window goes with it, timers
stopped. Verified by RC.EXE for console programs; RapidR applies it to GUI
programs too (`tests/conformance/cases/main_ends_after_show.bas`,
`docs/manual/components.md:93`; the probe was a console program, GUI programs are
not run in the VM). A person expects the window to stay; the ledger lists the
neighbouring mistake, a window never shown (row 11).
Where: the three hosts' end-of-main handling (desktop VM host, native runner, web
host); the model is shared. Recommendation: CHANGE: in a `.rr`, when the main
code ends while a window is visible, the program waits (events, timers) until the
last window closes; `END` still ends at once. M (three hosts, tests), low.
Importer: R, exact: add `END` after the main code when the program calls
`.Show` (not `ShowModal`) at top level, so the old program ends where it did.

### Left as they are (rows 39-47)

**39.** `TIMER` is the seconds since local midnight, a number a program can use
as it is. **40.** String `-` (`"jello" - "l"` is `jeo`): documented, harmless,
no one trips on it. **41.** `$DEFINE` in any case: the language is case-insensitive;
consistent. **42.** A FUNCTION's own name is a call: the usual BASIC rule; `RESULT`
is the value. **43.** A TYPE's PROPERTY SET is not called from the TYPE's own
code: avoids endless recursion, what a person expects. **44.** `DIM m` with no type
is a DOUBLE, `STRING * n` pads to n, `DIM a(3)` has 4 elements (0 to 3), parameters
are BYVAL (the safe default). **45.** `FILEEXISTS("*.txt")` matches a wildcard
(returning 1: see row 1); `Clipboard.GetAsText(n)` gives n - 1 characters
(n includes the end byte). **46.** The name rules (a data type's name, a name
twice, a control and an array of one name) are compile errors in RC.EXE's words,
not silent. `SUB Step` and friends are harmless. **47.** `...` closing a DECLARE,
a DECLARE's parameter names, `$INCLUDE "file` with no closing quote: needed by
RapidQ's own includes and examples, harmless.

## Not audited (components), and things found on the way

You asked to leave Q components' look alone, so no row covers a component's
size, colour, font or drawing. Component *behaviours* that read as surprises are
listed here without a recommendation, for a later pass if wanted: a status bar's
`SimpleText` shows only with `SimplePanel = 1`; a `Close` (or ModalResult) in a
form's `OnShow` under `ShowModal` is lost; `Form.Pixel` is -1 over child windows;
a canvas shows its parent's colour where nothing is drawn; an event handler's
STRING parameter keeps the last call's value (RapidR gives ""); a list box
has no `TabStop` or `SetFocus` (`L.TabStop` is a compile error in RapidR as in RC.EXE;
RC.EXE gives no component a TabStop and RapidR only RPanel and RLabel, an
accessibility gap that is additive to fix).

Not RC.EXE quirks, but met while reading:

- `REDIM a(5)` without the type on a STRING array turns its items into numbers
  (probe: `a(1)` reads 0); the ledger (row 12) lists the question as open. A
  RapidR defect either way.
- `ON ERROR` is accepted and ignored (`docs/manual/compatibility.md`): a user can
  believe errors are handled. In a `.rr` it should be a compile error or warning.
- `Left` and `LEFT$` are one name in RapidR, two in RC.EXE
  (`docs/errors-audit-2026-11.md`, "gaps left on purpose").
- The classic side stays as exact as before; nothing here asks to loosen it.

## Part 2: rows found by lanes (a `.rr` default to change)

Each row says the classic way (probed), what a `.rr` program does today and the recommendation; Robert decides.

| # | Item | Classic (probed) | A `.rr` program today | Recommendation | Status |
|---|---|---|---|---|---|
| 1 | The rhythm of an `RTimer` | The system's timer: ticks every Interval rounded up to 15.625 ms steps, **phase-locked** — the next tick is one period from where the last was *due*, not from where its handler ended; a tick that fell due during a long handler waits and runs the moment it ends (at most one waits). Probed in the Windows 11 VM with a handler spinning 0, 24, 50 and 130 ms: `Interval = 20` with a 24 ms handler gave gaps of 31 ms; with 50 ms, 50 ms back to back; `Interval = 100` with 24 ms gave 109 to 116 ms | The next tick comes one period **after the handler ended** (so `Interval = 20` with a 24 ms handler ticks every 55 ms, 18 a second; an emulator frame timer runs slow and its sound queue runs dry). Opt in to the steady beat with `FixedRate = 1` (every Interval exactly, a late tick made up at once, up to 250 ms behind) | Make the `.rr` default the steady beat: **`FixedRate` true unless set to 0**, so `Interval = 20` means a tick every 20 ms by the clock, as a person reads it. The manual already described the timer as "every Interval milliseconds"; today it is not. Cost: a program tuned to the slow rhythm ticks faster (its handler runs once a turn, so the window stays responsive); a conformance pair `.bas` against `.rr` pins both | **APPROVED by Robert 2026-11-07 19:43**, done: FixedRate is the `.rr` default (RapidR Studio's two "a turn later" timers keep `FixedRate = 0`) |
| 2 | A CONST written again (`$OPTION WEAKTYPE`) | A classic program that turns `$OPTION WEAKTYPE ON` on may write a CONST again, with no error and the last value winning, even of another type (`CONST A = 1` then `CONST A = "x"` prints x); with it OFF (the default) any second CONST of a name is `CONST variable A already defined`. Probed in the Windows 11 VM (the files `weaktype_*` in `tests/conformance/cases`): the option counts at the line of the *second* CONST; `Public Const` / `Private Const` count; it lets a CONST take the name of a CONST, a DIMmed variable or array, or a SUB or FUNCTION before it, and no other kind of second declaration; a VB port of the Win32 constants starts with it ON, ends with it OFF and repeats names `RAPIDQ.INC` has | A `.rr` program refuses every repeated CONST (`CONST variable A already defined — it is declared in x.rr:3 already`), whatever `$OPTION WEAKTYPE` says: `.rr` has no such option (its constants are built in, and one file declares a name once). The classic way is now kept for `.bas`, `.rqw` and `.rqb` only | **Accept a repeated CONST of the SAME value silently** in a `.rr` (the same name, the same value, the same type: the second line changes nothing, and a constants file that two included files both carry then just works); **a different value or type stays an error that names both places** (`CONST A is 1 in a.rr:3 and 2 in b.rr:9 — a constant has one value; rename one`), so a name never means two things. No option to switch on. Cost: a typo that repeats a line is no longer reported (it changes nothing); the importer needs nothing (a classic file's repeated CONSTs run as before) | **APPROVED by Robert 2026-11-07 23:26**: in a `.rr`, a repeated CONST with the SAME value (compared as values: `&H10` = `16`) is accepted with a quiet Problems *note* ("declared twice with the same value; you can delete one", a one-click fix); a DIFFERENT value or type is an error naming both places with the fix (keep this / keep that); only CONST (DIM/SUB/FUNCTION twice stay errors); independent of $OPTION WEAKTYPE and include order (the C/C++ rule for identical #defines). Lane after 2.123.1 |

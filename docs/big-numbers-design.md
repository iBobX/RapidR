# Big numbers: design (BIG-NUMBERS-A, 2026-11-05)

INT64 and UINT64 came in 2.122.1 in a rush. They worked inside an
expression the compiler could see was 64-bit, and nowhere else: a UINT64
above 2^63 joined to text showed as negative, a 64-bit value put into a
VARIANT, an array of VARIANTs, a component's property or a JSON document
came back cut to 32 bits, and a FOR loop's end, SELECT CASE and INPUT went
through a DOUBLE (exact only up to 2^53). This lane makes 64-bit numbers
work **everywhere a number goes**, adds UINT32 and Jango's bit functions,
and lays the plumbing INT128, UINT128 and BIGINT (part B) fit into.
The surface-by-surface audit is [big-numbers-audit.md](big-numbers-audit.md).

## 1. Why 2.122.1's design couldn't get there

A `Value::Integer` is RapidQ's whole number: it holds 64 bits, but PRINT,
STR$, HEX$, `\`, MOD and the bit operators read it as RapidQ's 32 bits
(beyond: -2147483648, the x87's "integer indefinite", as RC.EXE prints
`PRINT 2147483647 * 2`). 2.122.1 kept INT64 / UINT64 values in that same
kind and had `rapidr_ast::numeric` rewrite, at compile time, every
expression it could *see* was 64-bit into `__str64`, `__cmp64`, `__bits64`
& co. Whatever the compiler couldn't see lost the type: a VARIANT, an
array of VARIANTs, a SUB's untyped parameter, a builtin's argument
(FORMAT$), a property, an event argument, the debugger, JSON, a database
column, the AI tools. Each surface would need its own rewrite, forever,
and part B's 128-bit numbers can't live in an `i64` at all.

## 2. The design: the value carries its type

`rapidr_value::Value` gets two kinds:

```rust
Value::Int64(i64)   // INT64 (and UINT32, see §4)
Value::UInt64(u64)  // UINT64
```

Both are 8-byte payloads: `Value` stays 16 bytes, Integer and Double keep
their fast paths. A typed store makes them (`__to_int64` / `__to_uint64`,
the VM's `ToNum` with the kind's code, native builds' `numeric::to_int64`),
and from there the value is 64-bit wherever it travels: a VARIANT, an
array, a TYPE field, a parameter, a property, an event argument, the
debugger, JSON. Every generic operation in `rapidr-value` (the one the
interpreter, the JIT's helpers, native builds and the web all call)
handles them:

| Operation | Rule |
|---|---|
| Show (PRINT, STR$, `&` / `+` with text, `to_string_val`, the debugger) | every digit; a UINT64 unsigned |
| `+ - *` | 64-bit, wrapping (as 2.122.1, as C's `uint64_t` and two's complement); the result is a UINT64 when either side is one, else an INT64; a DOUBLE on either side gives a DOUBLE |
| `/`, `^` | DOUBLE, as always |
| `\`, MOD | exact (in 128 bits) with each side's own sign, the result wrapped to 64 bits; by zero, "Division by zero" |
| `=` `<>` `<` `<=` `>` `>=` | exact: a UINT64 above 2^63 is above every INT64, `u > -1` is true; against a DOUBLE exact too (2^53 + 1 is greater than the DOUBLE 2^53) |
| AND OR XOR NOT SHL SHR | all 64 bits (SHR logical, the count's low 6 bits), as 2.122.1 |
| unary `-` | wrapping, the type kept |

A classic `Value::Integer` meeting an INT64 is read as the whole number it
holds. Nothing changes for a program without INT64 / UINT64 / UINT32: it
never makes one of the new kinds, and every classic path is untouched
(RapidQ-exact, docs/rapidq-ground-truth.md).

### Where a value can't be held: a clear error

A surface that holds fewer bits says so instead of cutting the number:
an SQLite column (64-bit signed) refuses a UINT64 above 9223372036854775807
with a message naming the value; the same rule for every place listed in
the audit. Numbers a component reads as pixels, counts or indexes are read
as numbers are there (a Width of 5000000000 is a Width beyond any screen,
as a DOUBLE of that size is today).

### Overflow: wrap, as RapidQ's own LONG analogue does

`+ - *` wrapped at 64 bits in 2.122.1 and still do; hashes and checksums
(FNV-1a, xorshift) depend on it. ABS of the smallest INT64 is that number
again (`ABS(-9223372036854775808)`), and so is `-x`: RC.EXE gives the same
for RapidQ's LONG (`l = -2147483648 : l = ABS(l) : PRINT l` prints
-2147483648, probe 2026-11-05), so a .rr program stays consistent with the
classic types. A DOUBLE stored into an INT64 is truncated toward zero
(out of range: -9223372036854775808, mirroring the classic types' integer
indefinite); into a UINT64 out of range 0; a whole number keeps its 64
bits (-1 into a UINT64 is 18446744073709551615). All as 2.122.1 documented.

### FOR on a 64-bit counter ends at the type's end

`FOR u = 10 TO 0 STEP -1` on a UINT64 would step from 0 to
18446744073709551615 and loop for ever; `FOR i = x TO 9223372036854775807`
too. A 64-bit counter's step that would pass its type's limit ends the
loop instead (`rapidr_value::wide::for_step`; the VM's ForStep falls
through to the loop's exit, native builds the same; the counter keeps its
last value).

## 3. Literals

The parser keeps RapidQ's reading in the AST (`&H123456789` is its low 32
bits, as RC.EXE reads it) and notes each `&H` literal's full value and each
whole number beyond 9223372036854775807 (`rapidr_ast::wide_literals`). Where
a literal meets a 64-bit or UINT32 value (`rapidr_ast::numeric`: an
operator's other side, a store, a FOR counter's start / end / step, a
SELECT CASE on one, a CONST AS INT64, an argument to an INT64 parameter),
it is read as written: `&HFFFFFFFF` is 4294967295 there (so `i AND
&HFFFFFFFF` keeps the low 32 bits, which 2.122.1 got wrong), `&H` up to 16
digits, and a decimal above 9223372036854775807 a UINT64. Elsewhere a
classic program sees what RapidQ sees; a decimal beyond 64 bits there is a
DOUBLE, as RapidQ reads it (it was text before: a latent bug).

`VAL` returns a DOUBLE, as RapidQ's; `u = VAL(s$)` into a 64-bit variable
is read exactly (the store of `VAL(x)` reads `x`'s text itself).

## 4. UINT32

A 32-bit unsigned whole number, 0 to 4294967295 (Jango, groups.io #38944:
DWORD is RapidQ's 32-bit *signed* type, and stays so). It is held as an
INT64 value: a store wraps to 32 bits unsigned (`u = -1` is 4294967295,
`u = &HF58ACBFB` 4119514107), and an expression with it is worked out in
64 bits, so `u * 2` doesn't overflow and PRINT shows every digit. Its
literals are read unsigned (§3), so `IF u = &HF58ACBFB` is true.

## 5. Jango's bit functions (RapidR's)

Bit 0 is the rightmost. On a classic value they work on its 32 bits, on a
64-bit one on its 64:

| Function | Result |
|---|---|
| `LOWORD(x)`, `HIWORD(x)` | bits 0-15, 16-31 (0 to 65535) |
| `LOBYTE(x)`, `HIBYTE(x)` | bits 0-7, 8-15 (0 to 255) |
| `GETBITS(x, start, count)` | `count` bits from bit `start`, as a number from 0 |
| `SETBITS(x, start, count, bits)` | `x` with those bits replaced by `bits`' low `count` bits |

`start` + `count` beyond the value's width, or a negative one, is an error
that says the range (never a silent cut). `GETBITS` / `SETBITS` keep the
value's type (a UINT64 stays one).

## 6. HEX$, BIN$, OCT$, FORMAT$ on 64 bits

- `HEX$` shows 8 digits as for every number, 16 when the value needs more
  than 32 bits (a negative INT64 needs all 64): UINT32 4119514107 is
  `F58ACBFB`, the INT64 -1 `FFFFFFFFFFFFFFFF`. (2.122.1 always gave 16.)
  RapidR's `HEX$(x, digits)` gives at least `digits` digits (a hash in 16).
- `BIN$` / `OCT$`: all 64 bits without leading zeros, as the classic ones
  show 32.
- `FORMAT$`: `%d` and `%x` take a 64-bit value whole (a classic number
  stays RapidQ's 32 bits).

## 7. The runtimes

- **Interpreter**: the VM's fast paths stay Integer / Double; the new kinds
  take the generic path (the same `rapidr-value` function every runtime
  calls). The bytecode gets two constant kinds (Int64, UInt64): a module
  that uses one, or a new builtin, says it needs 2.123.0
  (`Module::min_runtime`), so 2.122 names the version instead of failing.
  2.122.1's `__str64` & co. stay for its modules.
- **JIT**: two more kinds, `K::I64` and `K::U64`, both in an `i64`
  register. Compiled: `+ - *` (wrapping), the exact comparisons (signed,
  unsigned, or one of each: a negative signed side is the smaller), `\` and
  MOD (signed; unsigned while no side is negative; the smallest INT64 `\`
  -1 without a trap), AND / OR / XOR / NOT, the stores (`__to_int64` & co.
  from a whole number; from a DOUBLE through the interpreter's own
  conversion), constants, array elements, ABS, and a 64-bit FOR counter's
  step with its end check (the counter as it was, the loop's exit taken).
  Anything else — a 64-bit value against a DOUBLE, a negative side in an
  unsigned `\` — leaves for the interpreter at that instruction. The
  answers are the interpreter's with `RAPIDR_JIT=always` and `off`
  (`tests/same_as_interpreter.rs`, `edge_values.rs` with the 64-bit edge
  values, conformance's `vm-jit`). A hot INT64 / UINT64 loop (10 million
  turns of a hash and a sum): 3172 ms on 2.123's development build (its
  64-bit operators were host calls), 859 ms interpreted now, 30 ms
  compiled.
- **Native builds**: the same `Value` and the same functions; defaults,
  typed arrays' reads and the FOR step use the new kinds.
- **Web**: the same VM and `rapidr-value` in wasm.

## 8. Part B: INT128, UINT128, BIGINT

One more kind, boxed, since an `i128` would grow `Value` to 32 bytes:

```rust
Value::Big(Rc<Big>)   // enum Big { Int128(i128), UInt128(u128), Int(BigInt) }
```

It slots into the same places: a `NumKind` per type, a conversion
builtin, the generic operations' "wide" arm (which already works on
`i128` for `\`, MOD and comparisons), the literal table (which keeps the
literal's digits, any length), the constant kinds (one more tag), the
audit's surfaces. The promotion order is Integer < INT64 < UINT64 <
INT128 < UINT128 < BIGINT, DOUBLE beside them as today. No instruction
depends on a type's width (docs/vm-fast-design.md).

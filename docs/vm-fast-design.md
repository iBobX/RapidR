# The faster interpreter (VM-FAST): design

The speed release's phase 2 (2026-11-05). What the bytecode interpreter
does to run typed code several times faster, with the same answers bit for
bit, the same debugger and the same web runtime. The numbers before and
after are in [speed-audit-2026-11.md](speed-audit-2026-11.md) ("Phase 2:
the interpreter"); the JIT that comes next is designed in
[jit-design.md](jit-design.md).

## Where the time went

`s = s + i` in a FOR loop over INTEGER variables ran about 19 instructions
an iteration: 5 for the statement (`LoadGlobal s`, `LoadGlobal i`, `Add`,
`ToNum`, `StoreGlobal s`) and 14 for the loop itself (the step's sign
tested every time, the counter compared with the end, stepped and stored,
two jumps). Each instruction cost about 3 ns on an M4 Max: decode the
opcode, read operands through a bounds-checked `Result`, push and pop
16-byte `Value`s, match their kinds, clone constants (`Const::to_value`
copies a string literal every time it runs), drop what was popped. The
profile shows `exec_loop` itself at 81 % with `Value`'s operators, the
constant copies and `Value`'s drop beside it (they are in another crate,
not inlined). And every builtin call (`ABS`, `SIN`) went through the host
by its name: a hash lookup, `catch_unwind`, two string matches.

So the work is: **fewer instructions** for the same statement, **cheaper
instructions** (no stack traffic, no copies), and **builtins resolved
once**.

## The rule that keeps every answer the same

Every new instruction is defined as exactly one sequence of the existing
instructions — its *expansion*. `AddLK a, k` is `LoadLocal a; LoadConst
k; Add`; `ForTestL i, end, step, exit` is the 10 instructions the FOR
loop's test was. The VM runs a new instruction with a **fast path** for
the kinds a typed program has (two Integers, two Doubles, an Integer and a
Double) and, for anything else, the **generic operation on the `Value`s**
— the very functions the expansion's instructions call. So:

- a declared INTEGER that the debugger set to a string, a BYREF argument
  that wrote back text, an INTEGER FOR counter stepping by 0.5 (it holds
  Doubles: `Value` addition makes it one) — all still give what they gave:
  the generic path runs;
- the fast paths are the generic operation's own cases written out
  (`Integer + Integer` is `wrapping_add`, `Double < Double` is
  `x.is_nan() || y.is_nan() || x < y` as RapidQ's FCOM leaves a NaN), and a
  test compares every new instruction with its expansion on edge values
  (0, ±1, `i32` and `i64` bounds, ±2^53 ± 1, -0.0, NaN, ±∞, 1E300, 2.5,
  -2.5, booleans, strings, Null): same kind, same bits, same errors;
- nothing changes for old programs: the old instructions are untouched.

Why not raw `i64` / `f64` slots? A local or global stays a `Value`
(Integer and Double are held inline in the 16-byte enum, nothing is
allocated): the debugger's Variables, Watch and Set Value, BYREF write-back,
event arguments and `evaluate` keep working as they do, and a slot that
holds something unexpected is handled, not trusted. The kind check costs
one predictable branch. Unboxed machine registers are the JIT's job, where
they pay (jit-design.md).

INT128 / UINT128 (the big-numbers lane) fit the same way: a new `Value`
kind goes through the generic path of every instruction at once, and gets
a fast path where it is measured to matter. No instruction depends on a
type's width.

## Where the compiler uses the declared types

`rapidr-bcgen` keeps, per scope, the numeric kind of every declared scalar
(`DIM n AS INTEGER`, typed parameters; globals of the main program) and
picks the instructions from them:

- an operand that is a variable or a number literal is fused into the
  operation (no `Load`, no push): `L` a local slot, `G` a global, `K` a
  constant, `S` the stack (an expression computed before);
- `Add` is fused only when one side is known to be a number (a declared
  numeric variable, a numeric literal, a `%` `&` `!` `#` name): two
  strings keep the generic `Add`, whose `x = x + y` appends in place;
- the conversion of a typed store is fused into the store
  (`StoreLocalNum`, `StoreGlobalNum`): no `ToNum` after the store; a typed
  parameter's conversion on entry is one in-place instruction
  (`ConvLocal`, `ConvArgLocal`), which does nothing when the value already
  has the kind;
- a form that reads a leaf *after* the expression on the stack (`a + (b *
  c)` as `MulLL b, c; AddLS a`) is used only when that expression can't
  change the leaf: it calls nothing but the pure maths builtins (a call of
  the program's could change a global, or a BYREF argument a local).

## The instructions (rapidr-bytecode `Op`)

Operand modes: `L` u16 local slot, `G` u32 global (its name's string id),
`K` u32 constant index, `S` the value on the stack.

| Group | Opcodes | Expansion |
|---|---|---|
| Arithmetic with fused operands | `0xA0 + 16 × op + 4 × a + b` for `op` = Add, Sub, Mul, Div, `a`, `b` ∈ S L G K (`SS` is the plain op, `KK` none); MOD in seven forms (SL SK SG LL LK GG GK) | `<a>; <b>; op`, pushes the result |
| Compare and branch | `JumpIfNot{Eq,Ne,Lt,Le,Gt,Ge}{SS,LL,LK,SL,SK,GG,GK,SG}` | `<a>; <b>; cmp; JumpIfNot t` |
| Typed stores | `StoreLocalNum s, kind`, `StoreGlobalNum g, kind` | `ToNum kind; Store` |
| In-place conversion | `ConvLocal s, kind`, `ConvArgLocal s, kind` | `LoadLocal s; [ToNum ARG_ROUND;] ToNum kind; StoreLocal s` |
| Local copy | `MoveLocal d, s` | `LoadLocal s; StoreLocal d` |
| FOR | `ForTestL/G var, end, step, exit`, `ForStepL/G var, step, top` | the loop's test (the step's sign, `<=` or `>=`, `JumpIfNot`), the step (`var + step`, store, `Jump`) |
| One-index arrays | `AGet{L,G}{S,L,G}`, `ASet{L,G}{S,L,G} kind` | `Load arr; <index>; AGet 1` / `Load arr; <index>; <value>; [ToNum kind;] ASet 1` |

The FOR loop keeps its shape and its lines: the test stays at the FOR
line, the step after the body (on the body's last line, as before), so
the debugger stops where it stopped (Step Into shows the FOR line at each
turn, a breakpoint on it stops once). No new instruction spans two
statements: the line table, breakpoints, Step, Set Next Statement and the
error line are as they were.

A fused array access reads the array after its index: used only when the
index (and, for a store, the value) is pure, as above. The array is
borrowed in place (no `Rc` clone and drop), the index taken as an integer,
a one-dimension element found without the general offset; anything else
(several dimensions, a string indexed as a list, out of range) takes the
generic path. The elements stay `Value`s: the measured cost of an access
was the way to it, not the element (§ the numbers).

## Builtins resolved once

`Module` keeps a derived table (filled the first time the VM runs it, never
written to a file): for each string of its pool, whether it is a *pure
maths builtin* — `ABS`, `INT`, `FIX`, `FRAC`, `SGN`, `SQR`, `SIN`, `COS`,
`TAN`, `ATN`, `ASIN`, `ACOS`, `LOG`, `EXP`, `CEIL`, `FLOOR`, `ROUND`, `CINT`,
`CLNG`, `CDBL`, `CSNG`, and `SHL` / `SHR` with two arguments
(`rapidr_bytecode::builtins::PURE_MATHS`) — and its constants as `Value`s (a string literal is
shared, not copied each time; strings are copied on write). A pure builtin
is called directly: the same `rapidr_value::builtins` function both hosts
call, without the host, the name lookup, `catch_unwind` or the event check
(none of them fires events, waits or raises an error). A host opts in
(`Host::DIRECT_MATHS`); the desktop, Studio's session and the web runtime
do, a test host that answers SIN itself doesn't. A string added to the
pool later (the debugger's snippets extend a copy of the module) is past
the table's end and goes through the host as before.

## The loop that runs them

Fewer instructions were half of it; the other half was what each one
costs once LLVM has compiled the loop. The profiles (`xctrace` Time
Profiler, mapped onto the disassembly) showed the first version of the
fused instructions inside the one big `exec_loop` *slower per instruction*
than before: the function had tripled, LLVM had stopped inlining
`Vec::push` (68 calls out of line) and `Value`'s drop (349), and the
instruction pointer lived on the stack frame because one helper took its
address. So:

- **`run_hot`**: the instructions that compute (loads, stores,
  arithmetic, comparisons, jumps, calls, arrays, all the fused ones) run in
  a loop of their own, `#[inline(never)]`, small enough to compile tight;
  it returns at an instruction it doesn't run (a host operation, the code's
  end), which `exec_loop` runs. Under the debugger `run_hot::<true>` makes
  the debugger's look itself before each instruction — exactly the check
  `exec_loop` made (an evaluation's steps; `debug_point` outside the rest
  of the line it last looked at, or when the interrupt is raised) — so it
  too leaves only for a host operation. Each instruction is written once.
- **Nothing in its own frame that every arm borrows**: the Null a
  never-assigned global reads and the FOR test's 0 live in the `Vm`. As
  locals of `run_hot`, LLVM stored their address into a stack slot per arm
  at every entry (~150 stores) — invisible while the loop is entered once
  per host operation, 3 × the cost of the 10 M loop under the debugger
  while it was entered at every line.
- **The opcode byte itself** is matched (`rapidr_bytecode::raw`), not
  `Op::from_u8`'s table first: one load fewer on the way to the arm.
- **No drop call for a number**: overwriting or popping a number, a
  boolean or Null skips `Value`'s drop (`fast::put` / `discard`:
  `mem::forget` of a value that owns nothing); a number over a number of
  its kind writes only the payload.
- **Slow paths out of line**: an Integer with a Double, text, Null,
  conversions to the narrow types, errors — `#[cold]` functions, so each
  arm inlines only its two fast cases.
- **A typed store folded in**: an operation whose next instruction is a
  `StoreLocalNum` stores its result there itself (a store is never a
  statement's start; under the debugger only inside the rest of a line it
  has looked at, never in an evaluation): the value is never pushed.
- **Operands read with one range check** (an array of 2 or 4 bytes), and
  `ip` kept in a register (the string append helper takes it by value).

## Calls

`CallFunc` / `Ret`: arguments popped into the new frame and its locals
dropped by plain pops (inlined; `Vec`'s drain, resize and a slice's drop
are calls), the returned frame's parameters handed to `LoadArgOut` by
moving them (they were cloned on every return), a typed parameter's
conversion on entry one in-place instruction.

## Typed arrays: not yet `TypedArray` in the interpreter

NATIVE-ARRAYS gave native builds `rapidr_value::typed_array::TypedArray<T>`
(plain `f64` / `u8` … storage) for arrays its analysis proves are used
only by element. The interpreter keeps `BasicArray` for now, measured:
`s = s + a(7)` costs 5 ns more than `s = s + x` per turn, two dispatches
(the index, the read); the element's `Value` is a small part of that, so
typed storage would save little while the instruction dispatch remains —
and a `TypedArray` isn't a `Value`: the VM hands arrays whole to builtins,
event handlers and the debugger. The JIT is where raw storage pays (its
compiled code reads the `f64` directly): jit-design.md, tier 2, shares
NATIVE-ARRAYS' analysis and type there.

## Old and new bytecode

- A 2.122 `.rrbc` runs as it did: every old instruction is still there
  and means the same; it gets the resolved builtins and shared constants
  too.
- A module that uses a new instruction says so in its header:
  `min_runtime` = 2.123.0 (`rapidr_bytecode::FAST_OPS_SINCE`), so a 2.122
  runtime refuses it with "this program needs RapidR Runtime 2.123.0 or
  newer", not "unknown opcode". A module without them keeps 2.117.0.
  The runtime built from this code knows the instructions whatever its
  version number says until the release bumps it
  (`RUNTIME_FEATURES`).
- The format number stays 3 (the layout didn't change, only the opcodes).

## The web

The browser runs the same `rapidr-vm` and the same `rapidr-bcgen` (the
web Studio compiles in wasm): the new instructions are in both at once.
`Host::YIELDS` still ticks at every backward jump (the FOR step's
included), so a busy loop still lets the page repaint.

## Tests (the automated guard)

- `fused_ops_match_their_expansions` (rapidr-vm): every new opcode against
  its expansion on the edge values above, kinds and bits compared, errors
  compared.
- Conformance cases for each typed path (tests/conformance/cases/
  `vm_fast_*.bas`), run on the interpreter, native and the web, with
  RapidQ's rules: INTEGER overflow beyond 32 bits, `\` and `MOD` rounding,
  INT toward zero, -0, NaN comparisons, a fractional STEP on an INTEGER
  counter, a typed variable written by a BYREF call, mixed kinds.
- `tools/bench/vm_budgets.json` tightened to the new numbers.

# The JIT: design (for the next lane)

The speed release's phase 3. The interpreter is now 2.5–6 × faster than
2.122 on typed code (docs/speed-audit-2026-11.md, "Phase 2: the
interpreter"; docs/vm-fast-design.md), but a fused instruction still costs
2–5 ns, and a native build does the same work at 0.3–1 ns. This document is
the plan for compiling hot typed code to machine code with Cranelift while
the program runs (`rapidr run`, Studio's F5). Sections 1–11 are the plan
as written before the work; **§12 says what tier 1 is as built
(2026-11-05) and where it differs from the plan, and why**. The
dependency's review: `docs/legal/review-2026-11-05-cranelift.md` (OK with
conditions; C1 closed in its addendum: regalloc2's relicensing covers the
file whose header still says MPL).

## 1. Why a JIT, and what is left after phase 2

The profiles of phase 2 (`xctrace` Time Profiler, instruction level) put
the interpreter's time in three places, in this order:

1. **Dispatch**: the chain opcode byte → jump table → indirect branch,
   about 35 % of a tight loop's samples. Phase 2 cut the number of
   instructions per statement by 3–6 × (fused operands, FOR as two
   instructions, typed stores folded); what is left is one dispatch per
   operation.
2. **`Value` traffic**: every operand and result is a 16-byte `Value`
   (tag + payload) read from a slot, matched, written back; the kind check
   is cheap, the loads and stores around it are not, and the big loop
   keeps some of its own state on its stack frame (register pressure).
3. **Calls**: a frame of ~100 bytes, its locals vector, the arguments
   moved, the parameters handed back for BYREF — ~35 ns a call (fcalls).

A JIT removes all three for the code it compiles: no dispatch, numbers in
machine registers, a call between compiled functions a machine call.

| Program (`rapidr run`, CPU ms) | 2.122 | phase 2 | native | what a JIT can reach |
|---|--:|--:|--:|---|
| `loop10m` | 553 | 120 | 23 | ~25–40 (the loop in registers) |
| `loop10m_sub` | 559 | 91 | 14 | ~20–30 |
| `top_double` (own ms) | 404 | 113 | 17 | ~20–30 (globals: one load and store a statement) |
| `dsp_scalars` (own ms) | 310 | 123 | 5 | ~10–20 |
| `fcalls` (own ms) | 355 | 133 | 13 | ~20–40 (calls between compiled functions) |
| `dsp_voices` (own ms) | 787 | 241 | 8 (typed arrays) | ~20–40 with typed array storage |

(Native numbers: the same machine, NATIVE-ARRAYS merged. "A JIT can
reach": a first Cranelift tier without inlining or loop optimisations,
typically within 1.5–3 × of an optimising native compiler.)

## 2. What gets compiled

**Unit: a whole function** (a SUB, a FUNCTION, or `__main`), compiled when
it is hot, entered at its start or — for a long loop already running — at
a FOR loop's head (on-stack replacement, below). A whole function keeps
the bookkeeping simple: one entry, every exit back to the interpreter
through the same deoptimisation map.

**Input: the bytecode, not the AST.** Phase 2's fused instructions are
already typed three-address code: `AddLK a, k`, `JumpIfNotGeLK ph, 1.0, t`,
`StoreLocalNum x, Double`, `ForTestL i, end, step, exit`, `AGetGL arr, i`.
A function is compiled when all of its instructions are in the **compiled
subset**:

- loads, stores and conversions of locals and globals; constants;
- the fused and plain arithmetic, comparisons, compare-and-branch, AND /
  OR / XOR / NOT on integers, `\`, MOD, `^`;
- FOR (`ForTest*`, `ForStep*`), jumps, `GOSUB` / `RETURN` inside the
  function;
- one-index array reads and writes (`AGet*` / `ASet*`) and `AGet` / `ASet`
  with more indices;
- the pure maths builtins (`fast::DIRECT`: ABS, INT, SIN, SQR …) as calls
  to their Rust functions (`extern "C"` wrappers on `f64`);
- calls to other functions (compiled ones directly, interpreted ones
  through a trampoline into the VM).

Anything else — a host operation (properties, methods, PRINT, a builtin
of the host's), text, objects, `CALLFUNC` — makes the function stay
interpreted in the first version. Later tiers call out to the VM for
those (§ 6).

**Types: what the program declares, checked by what runs.** bcgen already
knows each local's and global's declared kind (`Scope::kinds`,
`global_kinds`). The JIT keeps a declared INTEGER / LONG as an `i64`
register, a DOUBLE as an `f64`, a BYTE / WORD / SHORT as an `i64` wrapped
on store — exactly `rapidr_value::numeric`'s conversions, which phase 2's
guard tests already compare with the generic operations. A variable that
holds something else at run time (the debugger set it to text, a BYREF
argument wrote a string, an INTEGER FOR counter stepping by 0.5 holds
Doubles) fails a **guard** and the function goes back to the interpreter
(§ 4). Guards sit where a value comes in: at entry (each typed parameter
and local), after a call (a global may have changed), on a global read
after anything that can run code.

**INT128 / UINT128** (the big-numbers lane): Cranelift has `i128`; the
same plan holds with an `i128` register kind.

## 3. When

Two counters per function, kept in the VM beside the code (not in the
`.rrbc`):

- **calls**: compiled after 1,000 calls (Fib(27) compiles after its first
  millisecond);
- **loop turns**: every backward jump (`ForStep*`, `Jump` back,
  `JumpIf*` back) counts; at 10,000 turns of one loop the function is
  compiled and the running frame moves into the compiled code at that
  loop's head (**on-stack replacement**: the loop's FOR test is an entry
  point; the frame's locals are read into registers there). `loop10m` and
  `top_double` (all in `__main`) need this.

Compiling a small function with Cranelift takes ~0.2–1 ms. The first
version compiles on the VM's thread when a counter fires (simple, the
program pauses for that millisecond once); the next one builds the IR on
the VM's thread (it reads the module) and has a worker thread make the
machine code (Cranelift's `Function` is `Send`; `Value`s are not and never
cross), installed at the next counter check.

Compiled code is per **module** and per **process**: nothing is cached on
disk (no file a program could plant, no stale code after an edit).

## 4. Back to the interpreter (deoptimisation)

Every place compiled code can leave — a failed guard, a run-time error, an
instruction outside the subset reached in a later tier, the debugger's
interrupt — has a **deopt record**: the bytecode `ip` it corresponds to,
and for each local slot and stack entry live there, where its value is (a
register, a stack slot, a constant) and its kind. Leaving means: write
each value back as a `Value` into the frame's `locals` / the VM's stack,
set the frame's `ip`, return to `run_hot`, which carries on from there. The
interpreter then does exactly what it would have done.

**Run-time errors** never happen in compiled code: `MOD` / `\` by zero, an
index outside a one-dimension array, a conversion's edge — compiled code
checks the condition and deopts to the faulting instruction, and the
interpreter raises the error with its line, its message and the debugger's
break-on-error stop, as today. The same for anything rare (an Integer
overflowing into a case the compiled code didn't specialise for).

A function whose guards keep failing (more than ~10 deopts) is marked "stay
interpreted".

## 5. The debugger

- **Under the debugger** (Studio's F5 with breakpoints, Step, Pause) the
  program runs in the interpreter, as now: `run_hot::<true>` already runs
  one instruction at a time after the debugger's look, and phase 2 made it
  fast too (fewer instructions). No compiled code while `debug_mode` is on.
- **A program run without the debugger, then paused** (Studio's Pause, a
  breakpoint set while it runs): the interrupt flag is read by compiled code
  at every loop back-edge and function entry; when it is set, the code
  deopts there and the interpreter (in debug mode from then on) serves the
  pause — the user sees the line, the variables (now `Value`s again), and
  can step. Compiled code for the program is dropped when debug mode turns
  on.
- Watches, Immediate, Set Value, Set Next Statement work on the
  interpreter's frames: unchanged.

## 6. Tiers (the order of the work)

1. **Tier 1 (the next lane)**: functions in the compiled subset, typed
   scalars in registers, guards, deopt, calls between compiled functions,
   OSR at FOR heads, the pure maths builtins. macOS arm64 and Linux /
   Windows x86-64. Acceptance: `loop10m`, `dsp_scalars`, `fcalls` within 3
   × of native; every conformance case the same (`rapidr run` with the JIT
   forced on for every function: `RAPIDR_JIT=always`, and off:
   `RAPIDR_JIT=off`, for comparisons).
2. **Tier 2**: typed arrays as raw storage. NATIVE-ARRAYS' `TypedArray<T>`
   (`rapidr_value::typed_array`) and its analysis (an array used only by
   element, LBOUND / UBOUND and same-typed parameters — rapidr-codegen-rust
   `typed_arrays`, to move into `rapidr-ast` so bcgen and codegen share it):
   such arrays held as `TypedArray` by the VM too, compiled code reading
   `f64` / `i32` / `u8` directly. (Phase 2 measured that in the
   interpreter the element's representation is a small part of an access —
   the dispatch is the rest — so the VM keeps `BasicArray` until the JIT
   can use the raw storage: `dsp_voices` 241 ms interpreted.)
3. **Tier 3**: calls out of compiled code into the VM for host operations,
   strings and objects (compiled code stays compiled around a `PRINT` or a
   property read); inlining small functions; loop-invariant code motion.
4. **The web**: no native code generation in a browser page; instead the
   same IR lowered to a WebAssembly module (`wasm-encoder`, Apache-2.0 WITH
   LLVM-exception) and instantiated through `WebAssembly.instantiate` (the
   page's CSP must allow `wasm-unsafe-eval`; the web bundle's CSP is
   checked in tests/web_bundle_csp.mjs). A separate lane after tier 1.

## 7. Security

- **W^X**: code is written into pages that are writable and not
  executable, then made executable and not writable before it runs; never
  both at once. Off macOS that is `cranelift-jit`'s own memory (mapped
  read + write, then `region::protect` to read + execute).
- **macOS**: the hardened runtime refuses executable memory unless the app
  is signed with `com.apple.security.cs.allow-jit` and maps it with
  `MAP_JIT`, writing through `pthread_jit_write_protect_np`. **That needs
  RapidR's own memory provider**: the stock `cranelift-jit` memory maps no
  `MAP_JIT` memory and is *killed* under the hardened runtime even with
  `allow-jit` (tested by the lawyer review, §7, and again here).
  `interpreter/rapidr-jit/src/native.rs` maps one `MAP_JIT` region (16 MB
  reserved), writes with this thread's write protection off and turns it
  back on (and invalidates the instruction cache) before anything runs.
  At start-up the JIT maps one `MAP_JIT` page as a probe: refused (EINVAL:
  hardened runtime without the entitlement) it says so in
  `rapidr run --verbose` and the interpreter runs everything — an error,
  never a kill. `tools/release/macos/rapidr.entitlements` (the CLI) and
  `runner.entitlements` (the interpreted runners) carry `allow-jit`;
  **never `allow-unsigned-executable-memory`**. Tested here (macOS 27.2,
  Apple silicon, ad hoc with `--options runtime`): with `allow-jit` the
  loop runs compiled, without it "JIT: off, this process may not make code
  executable … the interpreter runs everything", the same answers. RapidR's
  ad-hoc export signer (not written yet) must not set the hardened-runtime
  flag; a user who notarizes an exported app adds `allow-jit` (else it
  runs interpreted).
- **The code generator's release**: `cranelift-*` pinned to `=0.137.1`
  (the reviewed one); its optional features `disas`, `incremental-cache`,
  `souper-harvest`, `enable-serde`, `pulley` and `cranelift-jit`'s
  `wasmtime-unwinder` stay off — `tools/check_jit_features.py` (in
  `tools/regress.sh legal`) fails if one is turned on, if the code
  generator brings a crate the review didn't list, if regalloc2 isn't the
  reviewed 0.15.2, if a source file of its tree carries an MPL header other
  than the known stale one, or if it
  reaches the web runtime, `rapidr-runtime-core`, a wasm or a 32-bit x86
  build. Its sources are never patched or edited.
- **iOS / iPadOS** (RapidR's mobile plans): JIT is forbidden for apps:
  interpreter only.
- **The web**: wasm modules only (§ 6), inside the page's sandbox.
- **Windows 7 / 8 32-bit** (the old-PC builds): Cranelift has no x86-32
  backend: interpreter only. Windows ARM64: per Cranelift's support at the
  time, behind a test.
- **What compiled code can touch**: only the frame's slots, the module's
  globals and constants, typed arrays' storage (bounds checked in compiled
  code, deopt outside), and the fixed list of `extern "C"` helpers. No
  bytecode-controlled address is ever dereferenced: the bytecode is
  verified before it is compiled (operands in range, jumps to instruction
  starts, slot counts) — the same verifier makes the interpreter's checks
  cheaper later.
- **Untrusted programs**: compiled code is the program's own code made
  faster; it gets no capability the interpreter doesn't have.

## 8. Memory

Each compiled function: a few KB of code plus its deopt records. A cap per
process (16 MB of code by default); past it, nothing new is compiled. The
counters: two `u32` per function. Code is freed with the module.

## 9. Cranelift's licence and what it pulls in (for the lawyer check)

Cranelift (`cranelift-codegen`, `cranelift-frontend`, `cranelift-jit`,
`cranelift-module`, `cranelift-native`, part of the Bytecode Alliance's
wasmtime project) is licensed **Apache-2.0 WITH LLVM-exception**. The
LLVM exception concerns **Cranelift's own code embedded in the binary that
links it** (object form: Apache-2.0 §4(a), (b), (d) waived there; §4(c)
still applies to the source form, as in the SDK's vendored crates). The
machine code Cranelift *generates* for a user's program is not
Cranelift's code: it contains no part of it, lives in memory, is never
distributed, and there is nothing to attribute either way. RapidR's
binaries and every exported program list Cranelift's crates in their
notices anyway (`THIRD_PARTY_NOTICES.md`, `rapidr notices`). Patent grant:
Apache-2.0's. No GPL / LGPL / AGPL by the declared licences or in any file
read; **one stale MPL-2.0 header**: `regalloc2`'s `src/lib.rs` — a
leftover comment: regalloc2's pull request #15 (2021, with Mozilla's
sign-off) relicensed the whole crate, that file included (the review's
addendum, C1 closed). The notices say so beside regalloc2, which is pinned
to the reviewed 0.15.2. The tree as resolved in RapidR's `Cargo.lock` (the
review's §1 has the exact list):

| Crate | Licence | Role |
|---|---|---|
| cranelift-codegen, -frontend, -jit, -module, -native, -entity, -bforest, -control, -codegen-shared, -assembler-x64, -bitset 0.137.1; build time only: -codegen-meta, -assembler-x64-meta, -isle, -srcgen | Apache-2.0 WITH LLVM-exception | the compiler |
| regalloc2 0.15.2 (pinned) | Apache-2.0 WITH LLVM-exception (its `src/lib.rs`'s MPL header is stale: PR #15) | register allocator |
| target-lexicon 0.13.5 | Apache-2.0 WITH LLVM-exception | target triples |
| wasmtime-internal-core, wasmtime-internal-jit-icache-coherence 50.0.1 | Apache-2.0 WITH LLVM-exception | support; instruction-cache flush |
| gimli 0.33.0 | MIT OR Apache-2.0 | unwind info |
| arbitrary 1.5.0 | MIT OR Apache-2.0 | **in the tree** (`cranelift-codegen`'s `std` turns on `cranelift-control/fuzz`) |
| memmap2 0.9.12, mach2 (macOS), libm, foldhash, fnv, bitflags 1.3.2, equivalent, indexmap, anyhow, hashbrown, smallvec, log, bumpalo, libc, rustc-hash, windows-sys | MIT OR Apache-2.0, MIT, Zlib, BSD-2-Clause OR MIT OR Apache-2.0 | support |
| region 3.0.2 | MIT | page protection (W^X off macOS, the start-up probe) |
| serde (the `enable-serde` features) | MIT OR Apache-2.0 | off |

Done when the dependency was added (2026-11-05): `cargo deny check`
(advisories, bans, licenses, sources) ok on RapidR's `Cargo.lock`;
`THIRD_PARTY_NOTICES.md` regenerated (697 libraries, a new
"Apache-2.0 WITH LLVM-exception" group); `rapidr notices` lists the
crates for every desktop kind and none for the web; the web runtime,
the web host and `rapidr-runtime-core` don't reach them
(`tools/check_jit_features.py`).

## 10. The plan for the next lane

1. A bytecode verifier (`rapidr_bytecode::verify`): every module before it
   runs; its errors as "this program is damaged" (security, and the JIT's
   precondition).
2. `interpreter/rapidr-jit` (new crate, feature `jit`, off on web / iOS /
   32-bit): IR building from the fused instructions, guards, deopt records,
   the `extern "C"` helpers; the VM's counters and the hand-over in
   `run_hot` (a compiled entry for the frame's function, or for the loop
   head it is at).
3. Tests: every conformance case with `RAPIDR_JIT=always` and `off`, the
   phase 2 edge-value guard run through compiled code, deopt tests
   (guards failing at each kind of place), the debugger's pause into
   compiled code.
4. Budgets: `tools/bench/vm_budgets.json` gets the JIT's numbers; the
   interpreter's stay (`RAPIDR_JIT=off`), so a regression in either fails.
5. macOS signing with `allow-jit` (with the code-signing plan), a check at
   start, the fallback said.

## 11. The pillars

- **Fast**: the remaining 3–8 × to native for typed code, measured per
  tier.
- **Easy to use**: nothing to switch on; a program runs faster as it is.
- **Accessible**: the debugger's views are the interpreter's, unchanged.
- **Beautiful**: smooth audio and animation from interpreted programs.
- **Legal**: Apache-2.0 WITH LLVM-exception and permissive support crates,
  reviewed before the dependency is added; nothing on disk.

## 12. Tier 1 as built (JIT-TIER1, 2026-11-05)

Code: `interpreter/rapidr-bytecode/src/verify.rs` (the verifier),
`interpreter/rapidr-vm/src/jit.rs` (where the JIT plugs in: the VM has no
`unsafe` and no code generator), `interpreter/rapidr-jit/` (`compile.rs`
bytecode → IR, `engine.rs` counters, batches and the hand-over,
`native.rs` every `unsafe`: memory, the probe, the call, the helpers).
The JIT is the `jit` feature of `rapidr-vm-host-native`: on in the CLI
(`rapidr run`, so Studio's runs) and the interpreted runner; never in
the web host, `rapidr-runtime-core`, iOS, wasm or 32-bit x86 (there the
crate has no code generator and `install` says why).

### 12.1 The verifier

`rapidr_bytecode::verify::module`, run by `Vm::run` before the first
instruction (so on every host): every instruction decodes; every operand
names something the module has (a local slot below the function's
`n_locals`, a string, a constant, a function, a conversion kind a store
or `ToNum` knows); every jump lands on an instruction's first byte or the
code's end; parameters fit the locals; the line table stays inside the
code. A module that fails: "this program is damaged and can't run: <what>
(in <routine>, routine n, at byte b). Build it again from its source".
Tests: one hand-broken module per kind of damage, the message's place,
and every byte of a sound module changed to ten values — the verifier
never panics and what it passes decodes to the end. Every conformance
case passes it (no false alarm).

### 12.2 What gets compiled, and where it differs from §2–§4

- **Forms, not one function each.** An *entry form* is entered from the
  interpreter, at a loop's head (OSR) or at a routine's start; a *callee
  form* is called machine to machine by compiled code, specialised for its
  arguments' kinds (Fib(Integer), Clamp(Double, Double, Double)), and
  returns its value in registers. A loop's entry form covers the loop
  only (its head to its last backward jump): **a loop is compiled even
  when its routine has instructions outside the subset** (`loop10m`'s
  `__main` PRINTs; §2 would have left it interpreted). Leaving the region
  is an exit.
- **Kinds per point, not per variable.** A local is Null at a routine's
  start and a Double after `Result = lo`: each point of the code has its
  own kinds, worked out from the kinds the form was entered with (an entry
  form: the frame's values then; checked again at every entry) and the
  bytecode. Where two paths meet with different kinds, the second one
  leaves for the interpreter.
- **Every way out is an exit with a record**: where the interpreter goes
  on (the instruction compiled code stopped *before*: nothing of it has
  happened), each local and stack entry and its kind. A callee that leaves
  writes out its whole frame and returns 1; each caller up to the entry
  form leaves at the instruction after its call; the VM gets real frames
  for all of them, each above its caller, and the interpreter goes on in
  the innermost — returning, BYREF write-back and all, as if it had never
  stopped. Run-time errors (MOD / `\` by zero, an index outside an array,
  …) are exits before the instruction: the interpreter raises the error,
  with its line and message and the debugger's break-on-error stop.
- **Arrays through helpers**: one-index reads and writes call
  `rapidr_jit_aget` / `aset` (bounds checked there, the element's kind
  checked against the one compiled for). Compiled code dereferences no
  computed address at all: only the context's fields, the locals and
  globals buffers at constant offsets the verifier vouched for, and its
  own stack slots. (Tier 2 gives raw typed storage.)
- **The maths builtins** call the interpreter's own functions through a
  helper (`ABS` inline; SHL / SHR through a two-argument one); the
  result's kind is checked.
- **Left to the interpreter in tier 1**: text, objects, host operations,
  `GOSUB` / `RETURN`, `CALLFUNC`, a call whose BYREF arguments are
  written back (`LoadArgOut`), arrays of more dimensions.
- **Hotness**: 10,000 turns of a loop, 1,000 calls of a routine. The VM
  counts per routine (a countdown at every backward jump and call) and asks
  the JIT when it runs out. A loop whose form is entered 16 times makes the
  loop around it hot (a synth's voice loop around its sample loop: its own
  turns are few).
- **Not worth it**: a loop or routine with an instruction it leaves to the
  interpreter, or kinds that disagree, inside it is not compiled (it would
  leave at every turn). `RAPIDR_JIT=always` compiles it all the same, at
  the first turn or call — the tests' way: every exit taken, often.
- **Deopts**: a form whose values keep being of other kinds (more than 10
  times) is not used again.
- **Depth**: compiled calls inside compiled calls stop at 256 (the
  machine stack's room); deeper, the interpreter makes the call, and its
  stack overflow error is the one it always gave.
- Compiled on the VM's thread when a count runs out (0.1–2 ms a batch);
  per module and per process, nothing on disk; 16 MB of code at most.

### 12.3 The debugger (differs from §5)

Studio's F5 always runs in debug mode, so "no compiled code in debug
mode" would have left F5 interpreted. Instead: **compiled code runs under
the debugger while it watches nothing** — no breakpoint, no step, no
evaluation — and checks the debugger's interrupt at every loop turn and
every call it makes. Pause, a breakpoint set while it runs, any request:
the interrupt is raised, compiled code leaves at the loop's head (or the
callee's first instruction), and the interpreter's own look serves it —
the line, the variables as values, Step, Set Next Statement, all as
before. With a breakpoint or a step active nothing compiled is entered.
Tests (`interpreter/rapidr-jit/tests/debugger.rs`): a pause in a hot
loop stops on its lines with the counter as an Integer; a pause in a
compiled call; with a breakpoint nothing compiled runs; stepping after a
pause, then going on compiled to the right answer.

### 12.4 Tests and guards

- `interpreter/rapidr-jit/tests/edge_values.rs`: phase 2's edge values
  (24 in a release build, 10 in a debug one) through every fused
  arithmetic form, every plain operation, every compare-and-branch,
  every conversion, the maths builtins, the FOR test with five kinds of
  step, one-index array reads and writes: the same kind, bits, branch and
  error with the JIT as without.
- `tests/conformance/run.mjs`: the `vm-jit` backend runs every case with
  `RAPIDR_JIT=always` beside `vm` (`off`): all the same; 149 of 298 cases
  run compiled code.
- `tools/bench/vm_budgets.json`: `<name>_jit` beside the interpreter's
  (measured with `RAPIDR_JIT=off`).
- `tools/check_jit_features.py`: §7's pins and features, §9's tree.

### 12.5 Numbers (`rapidr run`, CPU ms, the fastest of 5; M4 Max busy with other lanes)

| Program | phase 2 (`RAPIDR_JIT=off`) | JIT | native (§1's table) |
|---|--:|--:|--:|
| `loop10m` | 123 | 16 | 23 |
| `loop10m_sub` | 99 | 16 | 14 |
| `top_double` (own ms) | 122 | 24 | 17 |
| `dsp_scalars` (own ms) | 125 | 10 | 5 |
| `fcalls` (own ms) | 135 | 6 | 13 |
| `calls` (Fib 27, CPU) | 33 | 7 | — |
| `dsp_voices` (own ms) | 245 | 44 | 8 (typed arrays: tier 2) |

`loop10m`, `dsp_scalars` and `fcalls` are within 3 × of native (§6.1's
acceptance). Others: `arrays` 77 → 24, `byte_array` 71 → 24, `dsp_sin`
43 → 18; nothing it can't compile got slower (`strings` 442 / 436,
`concat_build`, `mid_scan` the same).

Music Studio's synth (`~/projects/RapidRMusicStudio/repro/synth_bench.rr`,
a copy run interpreted, 32 voices, the share of a 10 ms block):

| Mode | phase 2 | JIT |
|---|--:|--:|
| A, per-voice filter (sample-major) | 24.9 % | 9.8 % |
| B, eight bus filters | 23.6 % | 7.3 % |
| C, voice-major scalars | 10.4 % | 1.2 % |

### 12.6 Seen working

- RapidR Studio (this branch's, run from the checkout with `--do` and its
  capture hooks — no screen control): F5 on `loop10m.rr` and
  `dsp_voices.rr` (Output: the answers, `ms 54` for the synth); F5 on a
  2-billion-turn loop, Pause after a second — stopped on the FOR line with
  `i = 644,253,559` and `s = 644,253,558` in Variables (compiled code had
  run 640 million turns), Step to the line in the loop, Continue
  (compiled again), Pause again later in the next round.
- macOS hardened runtime, ad hoc with `--options runtime`: with the
  `allow-jit` entitlement compiled; without it "JIT: off, this process may
  not make code executable …", interpreted, the same answers.
- RapidR Studio itself run with `RAPIDR_JIT=always` (every loop and
  routine of Studio's own code compiled at once, exits everywhere), doing
  the same walk: it found one bug — a call compiled to a routine whose
  result was never worked out, so never defined ("can't resolve symbol");
  fixed (only routines worked out and defined are called compiled; one the
  code generator refuses becomes a stub that goes straight back to the
  interpreter) and kept as a test.
- x86-64: the JIT's tests (every edge value) and the whole conformance
  suite both ways pass with an x86-64 build run under Rosetta on this Mac —
  the code generator's x86-64 backend, not only its ARM one.
- Linux ARM64 (the Ubuntu 24.04 VM, `tools/release/linux/jit-check.sh`):
  built there from the branch, the JIT's tests (every edge value) pass, the
  conformance suite both ways (668 runs) passes; wall time with / without
  the JIT: `loop10m` 54 / 327 ms, `fcalls` 21 / 460, `dsp_scalars` 26 /
  313, `calls` 15 / 87. The code generator's own memory (read + write, then
  read + execute) is what runs there. A system that refuses executable
  memory (SELinux's execmem) is the start-up probe's error case, not a
  kill — not seen on Ubuntu's defaults.
- Windows (`tools/release/windows-vm.sh jit`, and
  `jit Target=x86_64-pc-windows-msvc` for x86-64 under Windows' own
  emulation): see the lane's report for the runs; the commands build the
  branch in the VM and run the same checks.

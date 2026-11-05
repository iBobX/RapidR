# The JIT: design (for the next lane)

The speed release's phase 3. The interpreter is now 2.5–6 × faster than
2.122 on typed code (docs/speed-audit-2026-11.md, "Phase 2: the
interpreter"; docs/vm-fast-design.md), but a fused instruction still costs
2–5 ns, and a native build does the same work at 0.3–1 ns. This document is
the plan for compiling hot typed code to machine code with Cranelift while
the program runs (`rapidr run`, Studio's F5). It is a plan, not code: no
dependency is added until the lead says so and the lawyer review is in
`docs/legal/`.

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
  executable, then made executable and not writable before it runs
  (Cranelift's `cranelift-jit` with `region`); never both at once.
- **macOS**: the hardened runtime refuses executable memory unless the app
  is signed with `com.apple.security.cs.allow-jit` and maps it with
  `MAP_JIT`, writing through `pthread_jit_write_protect_np`. RapidR
  Studio's and the runtime's signing (docs: the code-signing plan) gets the
  entitlement; a build without it runs interpreted (the JIT checks at start
  and falls back, saying so in `rapidr run --verbose`).
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
LLVM exception removes Apache-2.0's attribution requirement for code
*compiled* by it: programs RapidR's JIT runs carry no notice; RapidR's own
binaries carry Cranelift's notice in THIRD_PARTY_NOTICES.md (regenerated,
as for every dependency). Patent grant: Apache-2.0's. No GPL / LGPL / AGPL
in the tree as known today:

| Crate | Licence | Role |
|---|---|---|
| cranelift-codegen, -frontend, -jit, -module, -native, -entity, -bforest, -control, -codegen-shared, -codegen-meta, -isle (build time) | Apache-2.0 WITH LLVM-exception | the compiler |
| regalloc2 | Apache-2.0 WITH LLVM-exception | register allocator |
| target-lexicon | Apache-2.0 WITH LLVM-exception (0.12.16 checked locally) | target triples |
| wasmtime-jit-icache-coherence | Apache-2.0 WITH LLVM-exception | instruction-cache flush (arm64) |
| gimli | MIT OR Apache-2.0 (0.32.3 checked locally) | unwind info |
| hashbrown, smallvec, log, bumpalo, libc, rustc-hash, anyhow, windows-sys | MIT OR Apache-2.0 | support |
| region | MIT | page protection (W^X) |
| arbitrary, serde (optional features) | MIT OR Apache-2.0 | not enabled |

To be confirmed when the dependency is added (versions move): `cargo
metadata` / `cargo deny check licenses` on the exact tree, the result and
the review in `docs/legal/review-<date>-cranelift.md` (memory:
lawyer-agent-review), THIRD_PARTY_NOTICES.md regenerated, the web runtime
untouched (no Cranelift in wasm).

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

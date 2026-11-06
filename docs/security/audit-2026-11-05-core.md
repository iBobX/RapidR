# RapidR security audit — CORE (2026-11-05)

Area **CORE** of the Preview 6 security audit (`dist/lanes/security-audit-brief.txt`):
the JIT that landed today (`interpreter/rapidr-jit`, its `unsafe` in
`native.rs`), the bytecode verifier (`interpreter/rapidr-bytecode/src/verify.rs`),
the VM (`interpreter/rapidr-vm`, the typed/fused ops, the JIT hand-over, stack
and frame limits), the interpreted runner stub, `rapidr-codegen-rust` (generated
`unsafe` / typed arrays), and the DLL / VARPTR / PEEK / POKE managed memory.
Audited `development` at `1e996749` (workspace 2.122.1). Earlier audits:
`docs/security-audit.md` (2026-10-08, SEC-14..SEC-20), `docs/security/audit-preview4.md`
(SEC-21..SEC-30); two merged audits took SEC-31..SEC-37. CORE's findings are
numbered **SEC-50 onward** (the lead's instruction, so they don't collide).

Nothing was run against any host, network or keychain. PoC and fuzz tests are in
the worktree (`interpreter/rapidr-jit/tests/`), `#[ignore]` so the build stays
green. Everything ran on this Mac's temp dirs only.

The headline: **the JIT is defensive and, tier 1, sound by construction** — the
VM carries no `unsafe`, every `unsafe` is isolated in `native.rs` with W^X, the
verifier gates operand ranges before anything compiles, and compiled code leaves
to the interpreter for every operation or operand it can't prove. A
verify-preserving bytecode fuzzer (§2) ran 6066 crafted modules through the JIT
and the interpreter with **no divergence and no crash**.
The one real weakness is the native-stack depth bound (SEC-50).

---

## 1. Findings

| ID | Sev | Area | One line |
|---|---|---|---|
| SEC-50 | **Medium** | JIT / native stack | Compiled recursion runs up to 256 native frames with no stack-overflow check (probestack off, no stack limit; the cap counts frames, not bytes) → a large-frame recursion crashes the runtime (SIGSEGV/SIGBUS) where the interpreter errors gracefully; a stack-clash risk with probestack off |
| SEC-51 | Low | VM / crafted bytecode | The legacy `Op::Redim` opcode allocates `n` elements straight from its `i32` immediate with no cap (unlike `NewArray`/`BasicArray::new`, capped at `MAX_ARRAY_ELEMENTS`) → a crafted `.rrbc` exhausts memory |
| SEC-52 | Info | JIT / defense-in-depth | Cranelift's IR verifier is off in release builds; for security-sensitive generated code consider keeping it on, at least early |

Nothing in CORE is Critical or High **as demonstrated**. SEC-50 should be fixed
before Preview 6 (see its "Blocking?" note): the reliable impact is a crash on
attacker-controlled input, and with probestack disabled the overflow is a
stack-clash candidate (potential memory corruption, not just a fault).

**Status (SEC-FIX-CORE, 2026-11-05):** SEC-50, SEC-51 and SEC-52 are all fixed
in `a73dcfdb` — each finding's "Fixed in" line below says how;
`docs/jit-design.md` §12.7 has the design and the numbers.

---

### SEC-50 — Medium — Compiled recursion overflows the native stack (no probe, no limit; the cap counts frames not bytes)

**Where.**
- `interpreter/rapidr-jit/src/engine.rs:32` — `const NESTING: usize = 256;`
- `interpreter/rapidr-jit/src/engine.rs:517` — `self.ctx.depth_limit = view.depth_room.min(NESTING) as i64;`
- `interpreter/rapidr-jit/src/compile.rs:1092-1096` — the only depth guard: compiled
  callees load `CTX_DEPTH_LIMIT` and exit (`Why::Depth`) when `depth >= limit`.
  The limit is a **frame count**; the frame's **size** is never considered.
- `interpreter/rapidr-jit/src/engine.rs:189-200` (`new_state`) — the code generator's
  flags: `opt_level`, `use_colocated_libcalls`, `is_pic`, `preserve_frame_pointers`,
  `enable_multi_ret_implicit_sret`, `enable_verifier`. **`enable_probestack` is not
  set (default `false`) and no stack limit is set** (`set_stack_limit` / a stack-limit
  global value is never used).

**What.** Every compiled callee is a real machine `call`; a compiled function that
calls itself (or a chain of compiled functions) nests real native frames up to
`NESTING = 256` deep before the depth guard sends the rest to the interpreter.
Because the guard counts frames and not bytes, and because Cranelift emits **no
stack probe** (probestack default is `false`, confirmed in the vendored
`cranelift-codegen-meta-0.137.1/src/shared/settings.rs:256-259`) and the engine
sets **no stack limit**, 256 nested compiled frames of a function with a large
frame simply run off the end of the thread's stack. A function's frame grows with
its live scalar locals: each deopt/exit writes the live locals into an explicit
stack slot of `8 × n` bytes (`compile.rs` `exit_now`, `create_sized_stack_slot`),
so a routine with thousands of live locals has a frame of tens of KB, and
`256 × (tens of KB)` exceeds the stack.

The interpreter never does this: it keeps frames on the heap (`Vm::frames`, a
`Vec`), drives calls with a loop (not Rust recursion), and stops at
`MAX_CALL_DEPTH = 100_000` with a graceful *"stack overflow"* run-time error. So
the same program is a clean error under the interpreter and a hard process crash
under the JIT.

**Attacker.** A malicious or merely pathological `.rrbc` the developer runs (a
downloaded example, a model-written program, a corpus file) — the desktop
attacker of the threat model. The JIT is desktop-only (never the web sandbox), so
this is a desktop-runtime weakness, not a web one.

**Scenario, step by step.**
1. The program defines a FUNCTION with many live scalar locals that recurses on
   itself (counting down), e.g. `deep(900)`.
2. It runs hot (or `RAPIDR_JIT=always`); the JIT compiles `deep` and calls it
   compiled.
3. Compiled `deep` recurses 256 native frames deep; the 257th call deopts to the
   interpreter — but the 256 live frames, each tens of KB, have already consumed
   more than the thread's stack.
4. A store in a deep frame's prologue/body touches the stack guard page → the
   process dies (SIGSEGV / SIGBUS). With probestack off and a frame larger than
   the guard region, the prologue can move the stack pointer *past* the guard
   without touching it — a **stack clash**: later stores land in whatever is
   mapped below the stack (heap, another thread's stack, the 16 MB `MAP_JIT`
   code region), i.e. memory corruption rather than a clean fault.

**Evidence (PoC, run locally).**
`interpreter/rapidr-jit/tests/poc_deep_recursion.rs`
(`compiled_recursion_overflows_native_stack`, `#[ignore]`). It re-execs itself as
a child that runs `deep` with the JIT forced on/off on a bounded-stack worker, and
the parent checks the child's exit signal.

- 1 MiB worker stack, 1200 locals, depth 900: interpreter `result=Ok(())`, **no
  signal**; JIT child **died with signal 10 (SIGBUS)**.
- 8 MiB worker stack (the real main-thread size on macOS/Linux), 6000 locals,
  depth 700: JIT child **died with SIGSEGV (exit 139)**; the interpreter completes.

So the crash is not an artifact of a small test stack — it reproduces on the
8 MiB stack the runtime actually uses, given a large enough frame.

**Recommended fix (for the Opus fix lane).** Any one of these removes the crash;
do the first two together for "super secure":
1. **Set a real stack limit.** Give each compiled function a stack-limit check in
   its prologue (Cranelift `Function::stack_limit` / a `GlobalValue` the engine
   fills with the thread's remaining stack), and on the limit **deopt to the
   interpreter** (as `Why::Depth` already does) rather than trap — so deep
   recursion continues interpreted and ends with the usual *"stack overflow"*
   error, matching the interpreter.
2. **Enable probestack** (`enable_probestack = "true"`, inline strategy) so a
   frame larger than the guard can never jump it — this closes the stack-clash /
   memory-corruption path even if a limit check is missed.
3. Alternatively/additionally, make `NESTING` size-aware: lower the compiled
   nesting cap for functions whose frame exceeds a threshold, or budget
   `depth_limit` from the compiled frame size against the thread's remaining
   stack.

**Blocking?** Recommend **fixing before Preview 6**. As demonstrated the impact
is a reliable crash (DoS) on crafted/pathological input — acceptable to defer for
many tools, but at odds with RapidR's "super secure / zero risk" posture, and the
stack-clash angle (probestack off) is a genuine memory-safety concern that is
cheap to close. Treat as a hard blocker only if a follow-up shows the clash
corrupts live memory; otherwise a high-priority fix. The interpreter is unaffected,
so `RAPIDR_JIT=off` is a complete mitigation in the meantime.

**Fixed in `a73dcfdb`** (SEC-FIX-CORE): recommendations 1 and 2 together, and 3
in effect. Inline stack probes in every frame over 4 KB (`enable_probestack`,
`probestack_strategy = inline`); before every compiled call one compare of the
stack pointer with `Ctx::stack_limit` — the higher of the thread's real stack
end (pthread on macOS / Linux, `VirtualQuery` on Windows) + a 256 KB margin +
the largest compiled frame, and the entry's stack pointer − the smallest
compiled frame × the frames the VM has room for — below which the interpreter
makes the call (`Why::Depth`: the same answer, or its own "stack overflow"
error at the same line). It replaces the 256-frame count (and the depth
counter); a check in the prologue was not used because Cranelift's traps.
Fib(32) 21.6 → 21.7 ms, loops unchanged. The PoC is now a regression test
(`poc_deep_recursion.rs`: both of its cases end with the interpreter's
answer, no signal), with `native_stack.rs` on the real main thread and a
runaway case; `rapidr run` of the 6,000-local PoC on the 8 MiB main thread:
SIGSEGV (exit 139) before, the interpreter's answer after. The JIT's tests,
these included, pass on macOS, Linux ARM64 (8,180 KB main thread) and
Windows 11 ARM64, whose main thread measured 1,014 KB free (the 1 MB
default): a smaller stack only hands calls back sooner, so no thread of a
known size is needed (`docs/jit-design.md` §12.7).

---

### SEC-51 — Low — `Op::Redim` allocates unbounded elements from a crafted immediate

**Where.** `interpreter/rapidr-vm/src/lib.rs:1885-1892` (the `Op::Redim` handler):

```rust
let n = read_i32(code, &mut ip)?.max(0) as usize;
…
let parts: Vec<String> = (0..n).map(|_| String::new()).collect();
*slot = v_str(&parts.join(","));
```

**What.** `n` comes straight from the opcode's `i32` immediate (up to
`i32::MAX ≈ 2.1 billion`), with **no cap**. `NewArray` and every real array go
through `BasicArray::new` (`crates/rapidr-value/src/lib.rs:279-294`), which caps
the element count at `MAX_ARRAY_ELEMENTS = 50_000_000` with a graceful *"array too
large"* error; the `Op::Redim` path does not. A `Redim` with `n = 2_000_000_000`
asks for ~2 billion empty `String`s (~48 GB) plus the joined string — an
out-of-memory abort.

**Attacker / likelihood.** `Op::Redim` (0x83) is a **legacy opcode the current
compiler never emits** (REDIM lowers to the `__redim` builtin, `rapidr-bcgen`
`lower_redim`, which goes through the capped array path). So this is reachable
only from a **crafted `.rrbc`**, not from RapidR source — hence Low. But the
verifier accepts it (operands in range) and the VM honours it, so it is a real
"crafted `.rrbc` with a bad operand" path, which is exactly CORE's remit.

**Evidence.** Code-level (the handler above). **Not executed** — it would attempt
a ~48 GB allocation; running it is the DoS. The contrast is `BasicArray::new`'s
cap, which this path lacks.

**Recommended fix.** Cap `n` in the `Op::Redim` handler against
`MAX_ARRAY_ELEMENTS` (or route it through the same checked allocation as
`NewArray`), returning the graceful "array too large" error. Cheaper still: have
the verifier reject an `Op::Redim` with an absurd immediate, since the compiler
never emits the opcode at all (a crafted-only opcode could even be rejected
outright). Consider the same sanity cap on `NewArray`'s computed extents before
`BasicArray::new` (it already caps, so this is belt-and-braces) and on very large
`n_locals`.

**Fixed in `a73dcfdb`** (SEC-FIX-CORE): the verifier refuses a `Redim` whose
count is below 0 or above `MAX_ARRAY_ELEMENTS` ("Redim makes N elements (from
0 to 50000000)", the routine and byte named); the VM caps it too, with
`NewArray`'s own "array too large (limit 50000000 elements)" error, and builds
the comma string directly instead of `n` empty `String`s. Tests:
`verify.rs` `a_redim_of_more_elements_than_an_array_may_have_is_damage`,
`rapidr-vm` `a_crafted_redim_is_capped_as_an_array_is` (run unverified through
`invoke_function` too). `NewArray`'s extents and `n_locals` stay as they are
(already capped by `BasicArray::new` and the verifier's `MAX_LOCALS`).

---

### SEC-52 — Info — Cranelift's IR verifier is off in release builds

**Where.** `interpreter/rapidr-jit/src/engine.rs:196` —
`enable_verifier` is `true` only under `cfg!(debug_assertions)`, i.e. **off in
release**.

**What.** Cranelift's IR verifier catches malformed IR the JIT *builds* (bad SSA,
type mismatches). The IR is produced by `compile.rs` from the program's bytecode;
a `compile.rs` bug triggered by crafted-but-verified bytecode could emit malformed
IR that a release build does not check, risking a miscompile (and compiled code is
security-sensitive). This is the standard trade-off (the verifier is a debug aid),
and the JIT-vs-interpreter differential tests are the real backstop against
miscompiles — but for a "super secure" posture early on, keeping the IR verifier
on (it is far cheaper than codegen) is worth considering for at least the first
releases, or gating it behind an env/flag that CI and cautious users can turn on.

**Recommended fix.** Optional: enable the IR verifier in release too (measure the
cost), or add a build/env switch. Not a vulnerability; a hardening note.

**Fixed in `a73dcfdb`** (SEC-FIX-CORE): measured on vm_bench's programs
(`RAPIDR_JIT=always`, compile time, fastest of 5) it adds **25 %** to compile
time (15.4 → 19.3 ms in all) — not negligible — so it is on in debug builds,
in `always` mode (conformance's `vm-jit` and the JIT's own tests, release
builds included) and with `RAPIDR_JIT_VERIFY=1`, and off in `rapidr run`'s
default. Test: `engine.rs` `the_code_generators_safety_settings`.

---

## 2. Fuzzing — crafted `.rrbc` through the verifier into the JIT

`interpreter/rapidr-jit/tests/fuzz_mutate.rs`
(`jit_matches_interpreter_on_mutated_bytecode`, `#[ignore]`). It compiles seven
seed programs (hot FOR loops, doubles + maths builtins, a one-index array
read/write loop, a recursive FUNCTION, a BYREF SUB, bitwise/boolean logic, nested
loops — the shapes the JIT compiles) and mutates their bytecode in
**verify-preserving** ways: retarget a jump to another instruction start, swap a
local / global / constant / conversion-kind operand for another in-range value,
and raw byte flips (the verifier filters the invalid ones). Every mutated module
that still passes `rapidr_bytecode::verify::module` is run under the interpreter
(`RAPIDR_JIT=off`) and the JIT (`RAPIDR_JIT=always`) and the output — and the
run-time error — compared. A divergence is a JIT correctness bug; a hard crash is
a memory-safety bug.

It runs the work in a **child process** this test re-execs of itself, so a
mutation that turns a loop endless (which both backends would then spin on) is
killed and skipped by the parent instead of hanging the suite, and one CPU core is
used, not many (important while other lanes build on this machine). Trials are
seeded per `(seed, iter)`, so they are independent and resumable.

**Result:** at 1500 iterations/seed (seed `0x5DEECE66D`), **6066 verified mutated
modules ran both ways with zero divergence and zero crash** (2021 endless-loop
mutations skipped by the kill-and-skip harness; 103 minutes on one core). A first
run at 60 iterations/seed gave 251 modules, also clean. This is empirical backing
for the static conclusion that the verifier is a sufficient precondition for tier 1
and that the JIT reproduces the interpreter exactly on crafted-but-verified
bytecode.

Knobs: `RAPIDR_FUZZ_ITERS`, `RAPIDR_FUZZ_SEED`. Run:
`cargo test -p rapidr-jit --test fuzz_mutate -- --ignored --nocapture`.

---

## 3. Checked and found SOUND (so the next audit need not redo it)

**The verifier** (`verify.rs`). Read end to end. It refuses a module whose
instructions don't decode, whose operands name something absent (a local slot
`>= n_locals`, a string/global, a constant, a function, a conversion kind the
runtime lacks), whose jumps don't land on an instruction start (or the code end),
whose parameters don't fit the locals, or whose line table points past the code.
Its own `mutations_never_panic` test guarantees a *passed* module decodes all the
way. It deliberately does **not** judge operand-stack height — the JIT works that
out per block and exits on a mismatch (below), and the interpreter checks each pop.
The things it does **not** bound — `U8` operands (argc, dimensions, `LoadArgOut`
param index), `Field` slots, the `Redim`/`NewArray` sizes — are all handled safely
downstream (the JIT bails to the interpreter; the VM uses `.get`/`.get_mut` and
`checked_*`), except SEC-51's `Op::Redim` size.

**The JIT is defensive about every operand it doesn't trust** (`compile.rs`
`walk_block`). Confirmed by reading each `bail!()` path: `GetField`/`SetField`
(`Field` slots), multi-index `AGet`/`ASet` and `NewArray` (`U8` dims),
`LoadArgOut`, `CallIndirect`, `GOSUB`/`GosubRet`, host operations, text and object
ops all fall to the catch-all `bail!()` (leave to the interpreter). It compiles
only scalars in registers, one-index arrays through bounds-checked helpers, the
pure maths builtins, and calls between compiled routines. `pop!()` on an empty
abstract stack bails; a block reached with different kinds or a different stack
height takes a `Why::Conflict` exit (`edge`), so every block is compiled for one
`(local-kinds, stack-kinds)` signature and SSA merges are consistent.

**The `unsafe`, all in `native.rs`.** The seven helpers' Cranelift signatures
(`compile.rs` `Helpers::declare`) match their Rust `extern "C"` definitions
operand-for-operand (cross-checked all seven). `rapidr_jit_aget`/`aset` reach
arrays by handle through `ctx.{garrays,larrays}.get(handle)` (bounds-checked) and
elements via `data.get(i-lo)` after an explicit `lo <= i <= hi` test, with
`try_borrow`/`try_borrow_mut` (no double-borrow panic). The out-slots the helpers
write (`aget`, `math`, `exit`) are 8-byte / `8×n` stack slots compiled code makes
for exactly that call. `conv`/`math`/`math2`/`aget`/`aset` wrap their work in
`catch_unwind` and return a status (a caught panic → the interpreter answers); a
panic in the unwrapped `rapidr_jit_exit` aborts cleanly (Rust aborts on unwind
across `extern "C"`), never UB. `call_entry`'s transmute matches `entry_signature`
and the target's C ABI. The globals/locals buffers compiled code reads/writes are
sized by the engine (`run`) to the module's counts and indexed at constant offsets
the verifier vouched for; no bytecode-controlled address is ever dereferenced.

**W^X** (`native.rs` `imp` on macOS). One `MAP_JIT` region, written only with the
thread's JIT write-protection off (`pthread_jit_write_protect_np`), made executable
(and the i-cache invalidated) in `finalize` before anything runs; a start-up probe
reports the hardened-runtime-without-entitlement case as an error, not a kill.
Compilation and execution are strictly interleaved on the one VM thread — the
engine compiles only from `loop_head`/`called` (between VM instructions), never
from inside a running helper — so there is no window where the region is writable
while compiled code runs, and no nested compile during a run. Off macOS the engine
uses the code generator's own memory (RW→RX).

**Deopt / exit correctness** (`engine.rs` `exits_sound` + `apply`). Before any
write-back, `exits_sound` checks every exit record is known, carries exactly as
many values as it names, references slots within the function's `n_locals`, has an
`ip` within the code, that the entry frame's record matches the top frame's
function, and that every reconstructed frame above the entry was left by an
`AfterCall`. `apply` then writes with `.get_mut`/`.get` (OOB-safe even if a check
were bypassed) and resumes the interpreter at `frame.ip` — which is exactly how the
VM resumes a caller after a return (`raw::Ret`/`raw::RetVal` set
`ip = frames.last().ip`), so the reconstructed `ip: next` is correct. Exit records
are produced by the compiler at real instruction boundaries, never injected by the
attacker. A malformed exit is caught and the form disabled, or (a "can't describe"
chain) the run aborts with a clear "please report / RAPIDR_JIT=off" panic — never
a half-written program.

**JIT gating.** `jit` is an opt-in Cargo feature (`default = []`), enabled only in
the CLI and the interpreted runner stub, and **never pulled by the web host or
`rapidr-runtime-core`** (confirmed: their `Cargo.toml`s don't name it, and
`tools/check_jit_features.py` passes — code generator 0.137.1 pinned, optional
features off, only the reviewed crates, not in the web runtime / runtime-core /
32-bit x86 / wasm). `install_mode` returns `Err` (interpreter only) on wasm/iOS/
32-bit and when the macOS probe fails. The verifier runs in `Vm::run` on **every**
host, JIT or not. Every path by which an attacker-controlled module reaches the
JIT goes through `Vm::run` first (verify): `rapidr run`, the runner stub, and the
session host all call `run`; `until_start` only configures the debugger before
`run` and execs no program code; `evaluate` (Immediate / set-value) runs with
`eval_fuel` set, which makes `jit_may_run()` false, so the JIT never touches the
unverified snippet module (and that module is compiler-produced from the
developer's own typed text, run in the safe-Rust interpreter).

**`rapidr-codegen-rust` and typed arrays.** `rapidr-codegen-rust` contains **no
`unsafe`** (the only two `unsafe` matches are a comment and a reserved-keyword
list) and emits none; `rapidr_value::typed_array` (native builds' raw storage) is
**zero `unsafe`** and every access is bounds-checked (`get1`/`set1` via
`checked_sub` + `.get`/`.get_mut`, `place2` via `checked_mul`/`checked_add` + a
`< len` filter, falling to the slow path/graceful error on OOB). `BasicArray`
indexing is `.get`-protected and caps at `MAX_ARRAY_ELEMENTS`. Generated Rust
string escaping was already reviewed sound in the 2026-10-08 audit (§2).

**`fast.rs`** (the ops the JIT mirrors and its helpers call). Defensive: `aget1`/
`aset1` bounds-check with `.get`/`.get_mut`; `call_direct`'s `code` is always valid
(from `direct_code`, and the helper wraps it in `catch_unwind`); `put`/`discard`
handle `Value` drops correctly.

**The runner stub** (`interpreter/rapidr-runner-stub`). The appended-payload and
macOS-bundle readers are bounded (`payload_len + FOOTER_LEN > len` rejected,
`read_exact` into sized buffers); it loads the program appended to its own binary
or named with `--bytecode` — the program's own code, verified by `Vm::run`. No
trust-boundary issue.

**The bytecode loader** (`io.rs`). `read_count` caps every item count against the
remaining buffer (no giga-allocation from a corrupt count), `n_locals` is capped at
`u16::MAX + 1`, `read_n` is bounds-checked, UTF-8 and const tags validated. (Prior
audit confirmed; re-confirmed here.)

**DLL / VARPTR / PEEK / POKE managed memory.** Reviewed in depth by SEC-19 /
C-SYS-2 (`docs/security-audit.md` §6): PEEK/POKE reach only RapidR's model, calls
are arity-checked and bounded, `RAPIDR_SANDBOX` refuses DLL calls, every block has
a `// SAFETY:` note. Nothing in this lane changed that code (it is in
`rapidr-runtime-core`/`rapidr-value`, which the JIT does not touch). Carried
forward as reviewed; a fresh pass is the SYSTEM area's, not CORE's new code.

---

## 4. Notes (not findings)

- **Interpreter resource exhaustion on crafted bytecode.** A verified module with
  a very large `n_locals` recursing deeply allocates one locals `Vec` per frame
  (`call`) up to `MAX_CALL_DEPTH`; and `NewArray`/`Redim` sizes drive allocations.
  These are graceful-ish (OS OOM or the `MAX_CALL_DEPTH` error) resource limits,
  not memory unsafety; SEC-51 is the one with no cap at all. A verifier sanity cap
  on obviously-insane sizes would be defense-in-depth.
- **`resume`/`invoke_*` don't re-verify.** They run the same module `Vm::run`
  already verified; the attacker can't supply a different module to them (the host
  — RapidR's own code — passes the one it ran). A one-line "verify once per module"
  guard in `Vm` (memoised) would make this robust against future host refactors.
  Informational.
- **Windows x86-64 ABI** for the entry forms and helpers is the one thing the
  Rosetta cross-run doesn't exercise (`docs/jit-design.md` §12.6); the signatures
  use the ISA's `default_call_conv`, which is WindowsFastcall there, matching the
  `extern "C"` helpers — but it is worth a real x86-64 Windows run before relying
  on it (noted in the design already).

---

## 5. Pillars

- **Fast** — untouched (audit only); SEC-50's fix (a stack-limit check that deopts)
  costs one compare in the prologue of compiled functions.
- **Easy to use** — a program that recurses deeply should see RapidR's graceful
  "stack overflow" error, not a crash (SEC-50).
- **Accessible** — n/a (no UI change).
- **Beautiful** — n/a.
- **Legal** — the JIT's dependency review (`docs/legal/review-2026-11-05-cranelift.md`)
  and `tools/check_jit_features.py` hold; no new dependency added by this audit.

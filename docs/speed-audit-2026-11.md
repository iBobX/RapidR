# Speed audit, November 2026

Step 1 of the speed release's SPEED-AUDIT lane (2026-11-05): what RapidR
costs today, measured, with the profiles that say why. No code was changed
for these numbers; the benchmarks and tools added are listed at the end.
Step 2 (the UI fixes) waits for the lead's go; each fix will add its
"after" column here.

## How it was measured

- **Machine**: Apple M4 Max, 16 cores, macOS 27.2; release builds of commit
  788fdd9c (`development`, 2.122.1). The Mac was shared with other lanes'
  builds (load average 10 to 70 while these ran): times are CPU time where
  a program runs alone (steady under load), and each wall-clock number is
  the median of many steps, taken twice when it mattered.
- **Interpreter**: `rapidr run` (the RapidR Runtime, the bytecode VM).
  **Native**: `rapidr build` (generated Rust, optimized).
- **Profiles**: macOS `sample` (1 ms), main thread, read with
  `tools/bench/sample_profile.py` (inclusive shares, callees, self time).
  A native build is stripped; its profile came from the generated project
  built with symbols (`CARGO_PROFILE_RELEASE_STRIP=none`).
- **Windows**: the UI benchmarks ran on real windows (the winit host,
  vello on the GPU) and on the headless host of a GUI test (no painting).
  Every run had `RAPIDR_REGISTRY` and `RAPIDR_PRINT_TO` in a temporary
  folder and `RAPIDR_AI_KEYSTORE=memory`; Studio ran with `--fresh`, its
  registry and data in a temporary folder.
- **Hand resizes** (real mouse input dragging a window's corner) were done
  in step 2, when the Mac was free, before and after the fixes (§6) — and
  RapidR Web Server's window too, from a copy whose start was stubbed (its
  own start runs `FpmStopAll` and `EngineStart` before the window shows).

## 1. The interpreter and native code

`python3 tools/bench/vm_vs_native.py --runs 5`: each program's CPU time
(the fastest of 5), its own `ms` line where it prints one (TIMER around the
work), and the interpreter / native ratio. The same answers on both.

| Program | What | `rapidr run` ms | native ms | interpreter ÷ native |
|---|---|--:|--:|--:|
| `loop10m` | 10 M iterations, top level (globals) | 551 | 23 | 24 × |
| `loop10m_sub` | the same in a SUB (locals) | 537 | 14 | 38 × |
| `top_double` | 5 M DOUBLE multiply-adds, top level | 404 | 16 | 25 × |
| `dsp_scalars` | 1.4 M synth voice steps, local DOUBLEs | 306 | 5 | 61 × |
| `dsp_voices` | the same with the voices in global DOUBLE arrays (~13 M element accesses) | 787 | 132 | 6.0 × |
| `dsp_sin` | 705,600 `SIN` calls into a global DOUBLE array | 557 | 428 | 1.3 × |
| `fcalls` | 2 M calls of a 3-argument DOUBLE FUNCTION | 340 | 14 | 24 × |
| `calls` | recursive Fib(27), ~630,000 calls | 55 | 5.6 | 9.7 × |
| `arrays` | 2 M INTEGER elements written, read, summed | 315 | 42 | 7.5 × |
| `byte_array` | 1 M BYTE reads with AND / SHR / SHL | 214 | 18 | 12 × |
| `strings` | 1 M rounds of MID$ / UCASE$ / + / LEN / = | 433 | 232 | 1.9 × |
| `concat_build` | `s = s + CHR$(x)`, 262,144 times | 34 | 10 | 3.4 × |
| `mid_scan` | `ASC(MID$(b, i, 1))` over 64 KB | 13 | 4 | 3.2 × |
| `stream_read` / `stream_write` / `binary_read` | 64 KB streams | 80 / 157 / 85 | 69 / 139 / 71 | ~1.2 × |

`vm_bench --no-windows` (the VM's budgets) met every budget, at a busier
moment than the table: start-up 9.8 ms, 13 MB, the 10 M loop 677 ms (791
under the debugger), Fib(27) 60 ms.

What this says:

- **The "7 × slower" is the array-heavy case.** Plain arithmetic and calls
  in the interpreter are **24 to 61 × native**; array code 6 to 12 ×; text
  and I/O 1 to 3 × (the work is in the runtime, shared by both).
- **`SIN` is slow everywhere**: ~600 ns a call in native code, tens of times
  what it should cost — it turns its argument into text on every call (hot
  spot 7).
- **Native typed global arrays are boxed**: the same synth loop is 132 ms
  with the voices in global DOUBLE arrays and 5 ms in local DOUBLEs (27 ×;
  hot spot 9). RapidR Music Studio's own benchmark found the same
  (0.956 → 0.052 ms a block).

Profiles (`rapidr run`, main thread):

| Program | Where the time goes |
|---|---|
| `dsp_voices` | `Vm::exec_loop` itself 77 % (the instruction loop, inlined ops), `pop_indices` 4.6 %, `BasicArray::offset` / `get` / `set` 6 %, boxed `Value` arithmetic (`Mul`, `Add`, `Sub`) 6 %, dropping `Value`s 2.3 % |
| `dsp_sin` | `rp_sin` **76 %** inclusive — float formatting (grisu, `format::Decimal`, `trim_zeros`), allocations, parsing back; the builtin call path (`call_builtin`) 80 % |
| `dsp_sin`, native | malloc / free / realloc and `mach_absolute_time` (called by macOS's allocator on free) about 70 %: the same text conversion |

## 2. The UI

### Property writes (RapidR Web Server's issue #12)

`tools/bench/ui/prop_writes.rr`, milliseconds for the whole loop (TIMER):

| Case | writes | `rapidr run` | native | wanted |
|---|--:|--:|--:|--:|
| #12's repro: 41 buttons, Left once each, form never shown | 41 | 12.0 | 10.9 | < 5 |
| the same on a shown form | 41 | 11.1 | 10.4 | < 5 |
| 200 controls on a shown form, Left once each | 200 | 263 | 239 | |
| the same 200, Left / Top / Width / Height (an OnResize relayout) | 800 | **1,055** | **971** | |
| 200 Caption / Text writes (not a place) | 200 | 131 | 124 | |

A place write costs **1.2–1.3 ms at 200 controls** and grows with the
control count; the interpreter and native pay the same (it is the runtime).
Profile (`rapidr run`, the 800 writes in a loop): `layout::after_set` →
**`scroll::update` 99.8 %** — every Left / Top / Width / Height / Visible
write makes the parent form work out its scroll bars again from all its
children: `get_children_of` (a scan of every component) and ~8
`rp_comp_get` reads per child. Inside those reads: `to_lowercase` 26 %,
SipHash 11 %, malloc / free 21 %.

### Resizing

`tools/bench/ui/resize200.rr`: a form with 200 controls (a toolbar, a
status bar, a side panel, a client panel with a 132-control grid; Align and
Anchors, no program code on resize) resized 120 times by the program
(ClientWidth, ClientHeight, then DOEVENTS: layout, events, paint).
`tools/bench/ui/resize_canvas12.rr`: 12 canvases the program lays out in
OnResize (48 writes) and paints in OnPaint (a grid, bars, text), as a
dashboard does. Milliseconds a step: median / 95th percentile.

| Case | `rapidr run`, real window | native, real window | `rapidr run`, headless (no paint) | budget |
|---|--:|--:|--:|--:|
| 200 controls: a frame | **28.8 / 50** | 10.8 / 19.5 | 3.9 / 4.3 | 16 |
| … of it, the two size writes (layout) | 2.03 | 0.46 | 1.98 | |
| 12 canvases: a frame | **28.1 / 31.2** | 26.9 / 30.2 | 14.7 / 23.2 | 16 |
| … of it, the program's OnResize (48 writes) | 8.5 | 7.9 | 8.5 | |
| … of it, the 12 OnPaints | 12.1 | 11.8 | 5.8 | |

Profiles (real window):

| Case | Where the time goes |
|---|---|
| 200 controls, `rapidr run` | the kernel's paint (`FormUi::paint_into` / `paint_node` / `sync`) **73 %**, almost all of it reading properties through `RtStore::get` → `rp_comp_get` (82 % of the frame: `to_lowercase` 24 %, SipHash 9 %, malloc and free 41 %); the size writes (realign, reanchor) 7 %; `sync_desk` 6 %; the GPU encode (vello) 2 % |
| 200 controls, native | the same shape: paint 64 %, `rp_comp_get` 70 %, `to_lowercase` 21 %, malloc 19 % |
| 12 canvases, `rapidr run` | `scroll::update` from OnResize's 48 writes **27 %**; canvas text (`text::clipped_text`: glyph outlines rasterized per `TextOut`, at 1 × and 2 ×) 15 %; `HiRes::fill` 13 %; `Bitmap::resize` 9 %; property reads ~34 % |

Drawing itself is cheap: the frame is spent asking the component registry
for the same properties again and again. The interpreter process pays 2 to
4 × more than a native build for the very same runtime work (the 200-control
form: 2.03 vs 0.46 ms for two writes, 28.8 vs 10.8 ms a frame, with the
same functions in both profiles and a larger share in malloc): to look at
again once the reads stop allocating (hot spot 11).

### The window loop with many components

`lists::pre_paint` runs at every turn of the window loop and asks every
component of every shown form for its children: `get_children_of` scans and
sorts the whole registry each time, so a turn costs O(N²) in the number of
components. In RapidR Studio (hundreds of components), while the caret
moves: **11.7 % of all the main thread's time** — more than half of its busy
time — is `pre_paint` → `get_children_of`.

## 3. RapidR Studio's editor

`tools/bench/ui/studio_caret.py` runs Studio on the headless host (no
painting) with a file in its code view, clicks in the text, then plays a
scenario; the cost is CPU time against the same run without it. Files: a
generated 4,967-line RapidR program (`gen_source.py 5000`); RapidQ's
`windows.inc` (17,832 lines, 973 `$DEFINE`s) and `rapidq2.inc` (8,501
lines) from the local RapidQ install — not in the repository — and a
5,000-line program that `$INCLUDE`s both (as a user's RAPIDQ.INC +
WIN32API.INC programs do).

| Scenario | CPU | wall | budget |
|---|--:|--:|--:|
| Studio's start with a 10-line file | 1.46 s | | |
| … with the 5,000-line program: open | +0.7 s | | |
| … with `windows.inc` (17.8 k lines): open | **+23.8 s** | | |
| … with the 5,000-line program that includes both: open | **+20 s** | | |
| a caret move (Down), 5,000-line program | 10.5 ms | 55 ms (the script's 50 ms floor) | 16 incl. paint |
| a caret move, `windows.inc` | 10.2 ms | 60 ms | 16 |
| an edit round (a letter typed and taken back, a 1.5 s pause, Down), 5,000-line program | **388 ms** | 1.69 s (the pause included) | 16 of UI-thread time |
| the same in the program that includes `windows.inc` | **3.3 s** | 3.5 s (the pause included) | 16 |
| `rapidr check` of that program | 5.0 s | | |

So the pause users reported is real: **after every pause in typing,
Studio analyses the whole program again on the UI thread** — 0.4 s for
5,000 lines, 3.3 s with RapidQ's big includes — and the next key or caret
move waits for it (in the big case the round took 2 s longer than its
scripted pause). Opening such a file costs 20–24 s.

Profiles:

| Moment | Where the time goes |
|---|---|
| an edit's pause (program + includes) | the language service 79 % of the main thread: the code editor's `lang::diagnose` → **`semantic_tokens`** (the whole program's colours) 38 %, Studio's **Outline** (`Lang.Outline`) 41 %; inside both, **`SourceFile::line_col` 77 %** (it counts the newlines from the file's start for every span it places) |
| opening `windows.inc` | `line_col` 40 %, the preprocessor 54 % — `$DEFINE` substitution 26 % (every line searched once per define) — the outline 52 %, the parser 58 % (inclusive, nested) |
| a caret move, no edit | 79 % idle; of the busy part, `pre_paint` → `get_children_of` more than half; the editor's own key work 0.3 % |

The editor component itself is fast: `editor_bench --quick` (20,000 lines)
meets every target (typing p99 0.015 ms; open 24 ms), and
`codeeditor_bench --quick` (the view, 20,000 lines) too (typing p99 3.2 ms
key → pixels). With the language service (`--service --lines 5000`) typing
misses its p99: **21 ms** key → pixels (target 16; p50 4.9 ms).

Also seen: in the designer of a 5,000-line form file, an arrow-key nudge of
a component costs ~100 ms CPU.

## 4. The hot spots, ranked, and the fix for each

Ranked by what users feel (the UI first: Step 2's), then the interpreter's
and native code's for the next lanes. "Same behaviour" means the same
results in the same order: each fix is a cache, an index or a deferral,
never a different answer.

| # | Hot spot | Where | Cost today | Proposed fix | Expected |
|---|---|---|---|---|---|
| 1 | A place write re-does the parent's scroll bars from every child | `crates/rapidr-value/src/layout.rs:898` (`engine::after_set` → `scroll_update(parent)`), `crates/rapidr-runtime-core/src/scroll.rs:84` (`update`), `:57` (`children`) | 1.3 ms a write at 200 controls; #12 11 ms; 800 writes 1,055 ms; 27 % of the canvas frame | Mark the container's bars stale; work them out once, before anything reads them (ClientWidth / Height, the scroll properties, the kernel's sync before a paint) or at the turn's end | a write → µs; #12 < 1 ms (target 5); 800 writes ~10 ms |
| 2 | Every property read walks a chain of ~12 models, each allocating a lower-case copy and hashing it; `rp_comp_type` (another lower-case copy, a hash and a String clone) up to ~10 times a read | `crates/rapidr-runtime-core/src/object.rs:939` (`rp_comp_get`), `:1203` (`rp_comp_type`), `:59` (`COMPONENTS: HashMap<String, …>`, SipHash); read by the kernel's paint through `src/ui/kernel_store.rs:30` (`RtStore::get`) and `:57` (`font`), `crates/rapidr-ui-kernel/src/paint.rs:717` / `:786`, `tree.rs:249` | 64–82 % of a resize frame; about a dozen lower-case copies and String clones a read | One lower-case key and one type lookup a read, handed down the chain; the chain skipped for a plain visual component's stored properties (no model claims them); keys looked up without allocating (ASCII case folded in the hasher); a faster hasher for the registry maps (a small multiply-rotate hasher of our own, ~20 lines, or the `rustc-hash` crate, MIT/Apache-2.0 — the lead's call) | a frame's reads ÷ 5–10: the 200-control frame ~10 → ~3 ms native, 28 → < 10 ms in `rapidr run` |
| 3 | `get_children_of` scans and sorts the whole registry; the window loop's `pre_paint` calls it for every component every turn (O(N²)) | `crates/rapidr-runtime-core/src/object.rs:2544`; `crates/rapidr-ui-app/src/lists.rs:236` (`components`), `:276` (`pre_paint`), called from `crates/rapidr-runtime-core/src/ui/kernel.rs:430` and `:682`; also `layout.rs` `children_of`, `scroll.rs:57`, `kernel_store.rs` `children` | > half of Studio's busy time while the caret moves | A children index per parent (creation order), kept on create, Parent change and destroy; `pre_paint` visits only the components it serves (lists, grids, headers, trees: a set kept at creation) | `pre_paint` O(lists); children O(children) |
| 4 | `SourceFile::line_col` counts newlines from the start for every lookup | `crates/rapidr-preprocessor/src/origin.rs:102`; via `crates/rapidr-parser/src/tools.rs:69` (`locate`), `crates/rapidr-langsvc/src/navigate.rs:176` (outline items), `crates/rapidr-langsvc/src/editor.rs:249` (`semantic_tokens`), diagnostics | 40–77 % of every analysis in big files | A line-start index made once per file (on first use), binary search | the analysis ÷ 2–4 in big files |
| 5 | Every pause in typing re-analyses the whole program on the UI thread, more than once: the editor's colours, Studio's Outline and Problems | `ide/documents.inc:421` (`CodeChanged` → the 600 ms `Analyzer` timer) → `:183` (`AnalyzeActive`: `Lang.Update`, `FillOutline`, `FillProblems`); `crates/rapidr-ui-kernel/src/components/codeeditor/lang.rs:295` (`diagnose` → `semantic_tokens`); `crates/rapidr-studio/src/langsvc.rs:252` (`outline`) | 0.39 s a pause (5 k lines); 3.3 s with big includes; opening 20–24 s | One analysis per text version shared by colours, outline and problems; the included files' preprocessing and parse kept while they don't change; then the analysis on a worker thread, its result applied when it is still current (the editor never waits) | a pause's UI cost < 16 ms; opening a big include a few hundred ms |
| 6 | `$DEFINE` substitution: for every line, all defines sorted by length, the line lower-cased and searched once per define | `crates/rapidr-preprocessor/src/lib.rs:1757` (`substitute_defines_outside_strings`), called at `:1060` | O(lines × defines): 26 % of opening `windows.inc`; `rapidr check` of a program with it 3.5–5 s | Sort once when the defines change; per line, its identifiers once (a set), a define tried only if its name is there (the set grows with what a substitution brings in) — the same order, so the same text | that part ÷ ~100 |
| 7 | `SIN` and `COS` turn their argument into text on every call (to look for a comma-separated list) | `crates/rapidr-value/src/builtins.rs:312` (`rp_sin`), `:326` (`rp_cos`) | ~600 ns a call; `dsp_sin` 428 ms native, 557 ms `rapidr run` | Only a string argument takes the list path (a number never contains a comma) | `dsp_sin` native ~25 ms (the boxed array is the rest), `rapidr run` ~140 ms |
| 8 | The interpreter's instruction cost: a stack of boxed `Value`s, a `Result` per pop, `ToNum` after every typed store, globals cloned on load, array elements popped / borrowed / cloned, builtins found by name on every call (a HashMap, `catch_unwind`, a string match) | `interpreter/rapidr-vm/src/lib.rs:800–1240` (`exec_loop`), `:847` (`LoadGlobal`), `:1118` (`ToNum`), `:1186` / `:1199` (`AGet` / `ASet`), `:1479` (`pop_indices`), `:995` (`CallBuiltin`); `interpreter/rapidr-vm-host-native/src/lib.rs:54` / `:261`; `crates/rapidr-value/src/lib.rs:459` (`host_call`) | arithmetic and calls 24–61 × native; arrays 6–12 × | (the JIT lane) typed instructions and locals; builtins resolved to an index when the module loads, pure maths without `host_call`; typed arrays as plain vectors; then Cranelift for hot loops | toward native |
| 9 | Native typed global arrays are boxed: each element access clones the global `Value`, borrows the array and boxes the element | `crates/rapidr-codegen-rust/src/lib.rs:711` (`gv`), `:2928` (`rp_get`) | the synth loop 132 ms vs 5 ms with locals (27 ×) | (the fast-arrays lane) a typed global array (`DIM a(n) AS DOUBLE` / `INTEGER` / `BYTE`) as a typed static vector with typed accessors, as `TG_` does for scalars | toward 5 ms |
| 10 | Canvas text rasterizes every glyph's outline on every `TextOut`, twice (1 × and 2 ×); a resized canvas reallocates its pixels | `crates/rapidr-value/src/objects/text.rs:668` (`clipped_text`); `objects/bitmap.rs` (`HiRes::fill`, `Bitmap::resize`) | 37 % of the 12-canvas frame | A glyph cache (face, size, scale, glyph → coverage); pixel buffers reused when a canvas grows back | the canvas frame toward 16 ms |
| 11 | The same runtime work costs 2–4 × more inside `rapidr run` than in a native build | (to find) | the 200-control frame 28.8 vs 10.8 ms | Re-measure after 1–3 (most of the difference is allocation-heavy code); if it stays, profile the interpreter's process setup (threads, allocator) | `rapidr run` frames as native's |
| 12 | A designer nudge (arrow key) in a big form file | (not profiled) | ~100 ms CPU at 5,000 lines | Out of this lane: for the designer's own pass | |

## 5. Step 2, as proposed (the lead's go came the same day)

In this order, each with its before / after in §6 and its guard:

1. Hot spot 1 (scroll bars once a turn) — #12 under 5 ms for 41 moves.
2. Hot spot 3 (children index; `pre_paint` over its own components).
3. Hot spot 2 (cheap reads; no allocation per lookup; the hasher) — a
   200-control resize under 16 ms a frame.
4. Resize: one layout and one paint per frame for a live resize (the
   kernel's events coalesced), checked with a real drag on macOS.
5. Hot spots 4 and 6 (line index, define substitution) and 5 (one shared
   analysis per version; no whole-program analysis on the UI thread per
   pause) — caret moves under 16 ms in a 5,000-line file, edit pauses too.

Hot spot 7 is a small fix (both runtimes and the web share `rp_sin`) worth
~15 × on SIN-heavy native code: the lead decides whether it joins Step 2
or the fast-maths lane. Hot spots 8 and 9 are the JIT and
fast-arrays lanes' starting data; 10–12 are listed for the lead.

**The guard** (brief §7): `tools/bench/ui_bench` in the `vm_bench` style —
`prop_writes.rr`, `resize200.rr`, `resize_canvas12.rr` and
`studio_caret.py` held to budgets (41 moves < 5 ms; a 200-control resize
frame < 16 ms; a caret move and an edit pause in a 5,000-line file < 16 ms
of UI-thread time), failing loudly, run by `tools/regress.sh perf`; and the
new VM programs in `vm_bench` with budgets of their own.

**New dependencies**: none so far. A hasher crate (`rustc-hash`,
MIT/Apache-2.0) only if the lead prefers it to a small one of our own.

## 6. Step 2: the fixes, before and after

Each fix is its own commit; "before" is the commit before it, measured
back to back with it on the same Mac (shared with other lanes: load
average 15–55), `rapidr run` unless it says native.

**Where things stand** (the brief's acceptance first):

| | before | after | wanted |
|---|--:|--:|--:|
| #12: 41 moves | 11–12.5 ms | **0.11–0.25 ms** | < 5 ms |
| 200-control form, a resize frame in a window (median) | 29 ms (`rapidr run`), 11 ms (native) | **10.7–12.2 ms**, **8.3 ms** | < 16 ms |
| …dragged by hand: frames for 120 mouse steps | 60–71 (`rapidr run`) | **119** (one per step), both runtimes | every step |
| a caret move, 5,000-line file (Studio's CPU, headless) | 10.5–11.7 ms | **2.0–3.0 ms** | < 16 ms with the paint |
| an edit pause, 5,000-line file (CPU) | 372–388 ms | **52–76 ms** | < 16 ms: not yet (see below) |
| opening RapidQ's `windows.inc` in Studio (CPU) | 25.3 s | **3.3 s** | |
| RapidR Web Server's window, by hand: frames in the drag | 22 | **65** | |

| Hot spot (§4) | Status |
|---|---|
| 1 scroll bars per write | fixed (exactly, when nothing is scrolled and nothing aligned; else as before) |
| 2 property reads | fixed (no copies, one type lookup, one font walk); the faster hasher measured and left out |
| 3 children scan | fixed (index; the owner-draw turn serves only its own components) |
| resize coalescing | measured: nothing to coalesce (one size a frame already) |
| 4 line counts | fixed |
| 6 `$DEFINE` search | fixed |
| 5 whole-program analysis per pause | first part fixed (Studio's outline, FIELD$); the rest — the analysis off the UI thread — proposed as its own lane |
| 7 SIN / COS | fixed |
| 8, 9 interpreter, typed arrays | the JIT and fast-arrays lanes' (their benchmarks now have budgets) |
| 10 canvas text | open: the 12-canvas frame is 18.8 ms, its OnPaints' text and pixels |
| 11 interpreter process slower at runtime work | open: still 2–4 × (the 200-control writes 0.75 vs 0.19 ms native); same functions in both profiles |
| 12 designer nudge | open (the designer's) |

**Checked on the final build** (a frozen copy of the branch, so the suites
saw exactly what is committed): the conformance suite on both backends
(650 passed, 0 failed — `scroll_batched_writes` included), the GUI events
suite native and interpreted (all checks passed), RapidR Studio's 81
goldens (all match), Studio's flows on the desktop host (2,985 checks
passed; the 4 that fail — `ai-smart-hover`, `ai-smart-click` twice,
`ai-smart-click-block`, the Assistant's hover tip and Help search — fail
the same way on the build before this lane, 788fdd9c),
`vm_bench --check` (every budget met, idle 0 % CPU) and `ui_bench --check`
(every budget met); each fix's own crates' unit tests as it went in.

**Left for the lead:**
- An edit pause still costs 52–150 ms on the UI thread, now mostly the
  compiler's own checks (`rapidr_bcgen::semantic`, the ERRORS lane's
  crate); making it free means the analysis on a worker thread — a lane.
- `codeeditor_bench --service --lines 5000` (typing with the language
  service): p99 key → pixels 21 ms (target 16), not looked at here.
- The web runtime has its own copies of the three UI patterns
  (`runtime-web`'s scroll update per write, children scan, property
  reads): not changed in this lane (the fixes in shared code — 4, 6, 7,
  FIELD$ — reach it). A parity lane with web measurements.
- `tools/check_words.py` fails on five unused allowlist entries for
  `ROADMAP.md` since the roadmap's rewrite (commit edeed986), not this
  lane's.

### Hand resizes, before any fix

A real drag of the window's bottom-right corner (macOS, real mouse events:
120 steps at 60 Hz, 240 × 160 pixels out and back, ~2.4 s), the program's
own count of the times between its OnResize events (`RESIZE_BY_HAND`;
gaps of half a second or more — the hand at rest — left out). Two runs
each where they differed:

| Form | `rapidr run`: events, median / p95 ms | native: events, median / p95 ms |
|---|--:|--:|
| 200 controls (`resize200.rr`) | 71 and 60 events; 28.7 and 48.5 / 52.9 and 51.5 | 120 events; 20.7 / 29.6 |
| 12 canvases (`resize_canvas12.rr`) | 98 events; 24.3 / 26.9 (OnPaint 1,192 ms, OnResize 422 ms in all) | 100 events; 25.3 / 28.7 |

A frame every 16.7 ms would give one event per mouse step (120) at about
16.7 ms each.

### Hand resizes, after the fixes

The same drags with the build after all the fixes (the drag script posts a
mouse step every ~19.8 ms: 2.38 s for 120 steps):

| Form | `rapidr run`: events, median / p95 ms | native: events, median / p95 ms |
|---|--:|--:|
| 200 controls | **119 and 119**; 18.0 and 17.7 / 25.5 and 28.5 (before: 71 and 60; 28.7 and 48.5 / ~52) | **119**; 17.0 / 27.6 (before: 120; 20.7 / 29.6) |
| 12 canvases | **119**; 19.8 / 24.9; OnResize 25.5 ms in all (before 422) | **119**; 19.6 / 25.0 |

Every mouse step now gets its own frame: the window keeps up with the
hand, on the interpreter as on native code.

**RapidR Web Server's main window**, resized by hand: a copy of its
program in a temporary folder (the real project untouched), its start
stubbed so nothing runs — no server, no PHP, no process stopped, no
settings saved, `HOME` a temporary folder; checked: nothing listening, no
child process — with 12 sample sites, its own OnResize timed (`rapidr run`):

| | events in the drag | between events, median / p95 | its OnResize, median / p95 |
|---|--:|--:|--:|
| before the fixes | 22 | 101 / 214 ms | 72.6 / 186.6 ms |
| after | **65** | **35 / 54 ms** | **12.0 / 38.1 ms** |

(Its own layout code is now most of what's left of a frame: 12 ms of its
35.)

**RapidR Studio, a real walk**: a real window opening RapidQ's
`windows.inc` (17,832 lines), a click in the code, then 60 Down-arrow keys
(real key events, ~20 a second), the process's CPU over the keys (the
paint included):

| | the window shows after | CPU to open (Studio's start included) | a caret move (CPU, painted) |
|---|--:|--:|--:|
| before the fixes | 21.8 s | 23.8 s | 26.2 ms |
| after | **3.8 s** | **3.7 s** | **11.5 ms** |

### The guard

`tools/bench/ui_bench` (budgets in `tools/bench/ui_budgets.json`, run by
`tools/regress.sh perf`; `--check` exits 1 on a miss — it does on the
build before these fixes, on five of five measures tried) and three new
`vm_bench` programs with budgets (`dsp_sin`, `dsp_voices`, `fcalls`).
Fastest of the runs, milliseconds:

| | before the fixes | after | budget |
|---|--:|--:|--:|
| #12: 41 moves, form never shown / shown | 12.5 / 11.5 | 0.16 / 0.11 | 5 / 5 |
| 800 place writes, 200 controls | 1,067 | 1.9 | 40 |
| 200 caption writes | 131 | 1.1 | 30 |
| 200-control resize step, headless (its writes) | 4.0 (2.0) | 1.0 (0.66) | 2.5 (1.4) |
| 200-control resize frame, real window (its writes) | 29.2 (2.0) | 10.7 (0.75) | 16 (1.4) |
| 12 canvases: OnResize, headless / window | 8.5 / 8.4 | 0.39 / 0.42 | 3 / 3 |
| 12 canvases: resize step headless / frame in a window | 14.6 / 28.7 | 5.2 / 18.8 | 10 / 25 |
| `rapidr check`, 5,000 lines + 1,000 `$DEFINE`s (CPU) | 630 | 138 | 350 |
| RapidR Studio: a caret move (CPU, headless) | 11.7 | 2.0 | 8 (the brief: 16 with the paint) |
| RapidR Studio: an edit round (CPU, headless) | 372 | 52 | 200 |
| `vm_bench dsp_sin` / `dsp_voices` / `fcalls` | 557 / 787 / 340 | 157 / 796 / 348 | 300 / 1,100 / 500 |

The 12-canvas frame in a window stays above 16 ms: its OnPaints' text
(hot spot 10) and its pixels, the next thing to fix for canvas-heavy apps.

### Fix 1: scroll bars worked out once, when that can't differ

A write of a component's Left / Top / Width / Height / Visible made its
parent work its scroll bars out again from all of its components, at once
(`scroll::update`). Now the parent is marked and its bars are worked out
once, before anything reads them: the program's next read or method call
(`rp_comp_value`, `rp_comp_read`, `rp_comp_method`), a read or set of the
bars or of ClientWidth / ClientHeight, the container's first aligned
component, and the window loop's turn (before the windows take the
components' places and paint).

Only when that gives exactly what working them out after each write gave:
nothing in the container is scrolled (so no component moves when a range
shrinks) and it has no aligned component (laid out again, when a bar comes
or goes, from the ranges of that moment). Otherwise the bars are worked out
at once, as before — a first version that put them off always gave other
answers in a scrolled form and beside an alClient panel; the new
conformance case `scroll_batched_writes` (its `.expected` is the output of
the build before) holds both kinds to what they were.

| `prop_writes.rr` | before | after |
|---|--:|--:|
| #12's repro: 41 moves, form never shown | 12.4 ms | **0.25 ms** |
| 41 moves, form shown | 11.1 ms | **0.22 ms** |
| 200 Left writes on a 200-control form | 251 ms | **1.0 ms** |
| 800 Left / Top / Width / Height writes | 1,016 ms | **3.7 ms** |
| 200 Caption / Text writes | 130 ms | **3.0 ms** |

Checked: the unit tests of rapidr-runtime-core, rapidr-value and
rapidr-ui-app; the conformance suite on the interpreter (325 cases) and
`scroll_batched_writes` built native too; the GUI events suite, native and
interpreted (123 cases, all checks passed).

### Fix 3: a children index; the window loop serves only its own components

`get_children_of` scanned and sorted the whole registry for every call, and
the window loop's owner-draw turn (`lists::pre_paint`) asked for every
component's children at every turn. Now an index of every parent's
children (their Parent's text in ASCII lower case → creation order, key,
type) is made once and kept while the tree doesn't change: a revision
(`rp_tree_revision`) moves whenever a component is made or a Parent is
stored. `pre_paint` keeps, per shown form, the components it serves —
lists, combo boxes, grids, headers, trees — in the same depth-first order,
found again only when the revision moves (`Program::tree_revision`; a
program that can't tell, the web's, walks the tree as before). Same
children, same order, same owner-draw events in the same order; a test
compares the index with the old scan through made, parented, moved,
self-parented and number-parented components.

| | before | after |
|---|--:|--:|
| RapidR Studio, a caret move in a 5,000-line file (CPU, headless) | 9.4–9.9 ms | **4.0–4.1 ms** |
| RapidR Studio idle (`vm_bench studio_idle`: budget 1 %) | 1.24 % CPU (missed) | **0.00 %** |
| RapidR Studio running the idle form (budget 2 %) | 1.49 % | **0.00 %** |
| the 200-control resize frame | 28.8–29.2 ms | 28.9–29.0 ms (its cost is elsewhere: fix 2) |

Checked: the unit tests of rapidr-runtime-core, rapidr-ui-app and
rapidr-ui-kernel (with the new index test); the GUI events suite, native
and interpreted (all checks passed); the conformance suite on the
interpreter (325); RapidR Studio's 81 goldens (all match); Studio's flows
on the desktop host (the four web-build flows need the web runtime's
wasm, not built in this worktree).

### Fix 2: a property read that doesn't copy or look things up again

What the kernel reads at every frame (every property of every control)
went through `rp_comp_get`, which made a lower-case copy of the name and
of the property at almost every step and looked the component's type up
about ten times, then asked a dozen models, each making its own copy of
the name to find it. Now:

- `rp_comp_get` makes the lower-case property and key once (a copy only
  when there is an upper-case letter or a non-ASCII one —
  `rapidr_value::lower`, what `to_lowercase` gives) and looks the type up
  once; a model that only answers for its own types (the panels, Studio's,
  RAI's, RSecrets, RSystemInfo, the scroll bars) is asked only for them.
- The shared models' lookups (`objects`, menus, `rqlib`, archives, audio,
  Direct3D, headers, scroll bars, layout) find a name without copying it.
- A control's font (six parts, each its own or the nearest parent's) is
  read in one walk up the parents, not six (a test compares both ways on
  2,000 random chains, loops included).
- The kernel's store looks for `Font.Color` without making a copy.

Same answers: the same functions in the same order, minus the copies and
the repeated type lookups. A faster hasher was tried after this (SipHash
was 12 % of what was left): the writes 12 % faster, the frames unchanged,
so it was left out — the standard hasher's resistance to crafted keys
stays.

| | before | after |
|---|--:|--:|
| 200-control resize frame, `rapidr run` (median / p95) | 29.1 / 51.6 ms | **12.2 / 18.2 ms** |
| … its two size writes | 2.03 ms | **0.75 ms** |
| 200-control resize frame, native (fixes 1, 2, 3, 7 together) | 11.3 / 19.8 ms | **8.3 / 9.3 ms** |
| … its two size writes, native | 0.47 ms | **0.19 ms** |
| 12-canvas resize frame, `rapidr run` (fixes 1–3 together) | 28.2 ms | **18.1–18.5 ms** (OnResize 8.6 → 0.42 ms; OnPaint 12 → 11 ms: canvas text, hot spot 10) |
| 12-canvas resize frame, native | 28.0 ms | **18.2–18.4 ms** |
| #12's 41 moves, native | 10.4–11.5 ms | **0.16–0.31 ms** |
| 800 place writes, native | 949–973 ms | **1.7–1.8 ms** |

### Fix 4: a file's line starts, kept

`SourceFile::line_col` counted the newlines from the file's start for every
place it was asked for; every span the outline, the colours and the
problems place asks for one. Now each file keeps where its lines start
(made the first time, a binary search after). The same line and column
for every offset (a test checks every offset of nine texts against the
old count, CR LF and UTF-8 included).

| RapidR Studio (headless, CPU) | before | after |
|---|--:|--:|
| an edit round in the 5,000-line program (with fixes 2–3) | 280 ms | **100 ms** |
| an edit round in the program that includes RapidQ's big files | 2,910 ms | **220 ms** |
| opening that program (Studio's start included) | 21.0 s | **16.1 s** |
| `rapidr check` of it | 5.0 s | 5.0 s (its cost is the `$DEFINE`s: fix 6) |

Checked: the unit tests of rapidr-preprocessor, rapidr-parser,
rapidr-langsvc and rapidr-lsp (25 test programs, all passed).

### Fix 6: `$DEFINE`s tried only where their name is a word

For every line the preprocessor sorted all the defines by length, then
lower-cased and searched the line once per define (973 in `windows.inc`).
Now the defines' order is made once while they don't change (a `$DEFINE`,
`#Const` or `$UNDEF` makes it again), and a line tries, in that same
order, only the defines whose name is one of its words — plus those a
substitution's value brings in, and any whose name isn't a plain word.
The same text and the same origin map: a test runs both ways on 3,000
random lines and define sets (values that bring other names in, case
differences, quotes, dots, CREATE blocks, kept TYPE fields).

| | before | after |
|---|--:|--:|
| `rapidr check` of the 5,000-line program including `rapidq2.inc` + `windows.inc` (CPU) | 5.0 s | **0.87 s** (the same output; what's left is the compiler's own checks — `rapidr_bcgen::semantic`, the ERRORS lane's) |
| RapidR Studio opening that program (Studio's start included) | 16.1 s | **3.9 s** |
| an edit round in it | 220 ms | **185 ms** |
| opening `windows.inc` itself | 17.7 s | 14.1 s (the rest: fix 5) |

Checked: the unit tests of rapidr-preprocessor, rapidr-parser,
rapidr-langsvc, rapidr-lsp and rapidr-cli.

### Fix 5 (first part): Studio's outline without quadratic loops

After fixes 4 and 6, opening `windows.inc` still took 14 s: Studio's
`Lang.Outline` placed each of its thousands of items by counting the
newlines before it (`rapidr_studio::langsvc::line_col`, 37 % of the open),
and Studio's own code read the outline a row at a time with
`FIELD$(text, CHR$(10), i)` — which copied the whole text and cut every
row out of it for each row (51 %). Now:

- the outline's items get their lines from the text's line starts (a
  binary search; the same line as `line_col` for every offset, a test
  checks);
- `FIELD$` finds the field where it is: no copy of the text, no list of
  every field (the same answer for every text, delimiter and index, a test
  compares with the old one) — every program's FIELD$ loops gain;
- Studio's Outline and Quick Open read the rows one after the other
  (`INSTR` from where the last row ended, `MID$`), as FIELD$ did, in
  `ide/documents.inc` and `ide/panels.inc`.

| RapidR Studio (headless, CPU) | before | after |
|---|--:|--:|
| opening `windows.inc` (17.8 k lines; Studio's start included) | 14.1 s (25.3 s before any fix) | **3.3 s** |
| an edit round, 5,000-line program | 100 ms (388 before any fix) | **76 ms** |
| an edit round, the program including RapidQ's big files | 185 ms (3.3 s before any fix) | **152 ms** |
| a caret move, 5,000-line program | 4.0 ms (10.5 before any fix) | **3.0 ms** |

**Not done, and why (stopped here as the brief says when a fix outgrows
its share).** What an edit pause still costs on the UI thread is now mostly
the compiler's own checks for the Problems pane and the squiggles
(`rapidr_bcgen::semantic`, about 75 % of what's left; the ERRORS lane's
crate), not the parse: the second analysis (Studio's Outline beside the
editor's colours) is about a tenth of a pause, so sharing one analysis
would save ~10 ms. Moving the analysis off the UI thread is what would make
a pause free, but the editor's service is called synchronously from the
kernel and its analysis lives in the UI thread's state: that is a lane of
its own (an analysis worker with versioned results), proposed for the
lead. Caching the preprocessing of unchanged include files gains little
now that preprocessing is ~4 % of a check.

### Resize coalescing: measured, nothing to coalesce

Before changing how a live resize is handled, a hand drag counted how the
OnResize events come: 63 events for 120 mouse steps, only 1 of the 63
intervals under 3 ms (`rapidr run`, the 200-control form). macOS already
gives one size per frame, and the runtime lays out and paints each once:
there is no second size waiting in the same turn to drop. A frame costs
what it costs because of what it reads (fix 2), not because it is done
twice.

### Fix 7: SIN and COS of a number make no text

`SIN` and `COS` turned every argument into text to look for a comma (a
list of numbers). A number's text never has one: an Integer or a Double now
goes straight to the maths, the result bit for bit the same (a test
compares both ways over every kind of value and a sweep of 2,900 phases);
anything else takes the old path.

| `dsp_sin.rr` (705,600 `SIN`s) | before | after |
|---|--:|--:|
| `rapidr run` | 561–582 ms | **150 ms** |
| native | 429–436 ms | **17 ms** |

## Benchmarks and tools added in step 1

| File | What |
|---|---|
| `tools/bench/vm/dsp_sin.rr`, `dsp_voices.rr`, `dsp_scalars.rr`, `fcalls.rr`, `top_double.rr` | SIN-heavy DSP, typed global arrays, the same on local scalars, DOUBLE calls, top-level DOUBLE arithmetic |
| `tools/bench/vm_vs_native.py` | every `tools/bench/vm/` program on `rapidr run` and native, side by side |
| `tools/bench/ui/prop_writes.rr` | #12's repro and 200-control place / caption writes |
| `tools/bench/ui/resize200.rr`, `resize_canvas12.rr` | resize frame times (program-driven, or `RESIZE_BY_HAND=<s>` for a real drag) |
| `tools/bench/ui/gen_source.py` | a valid N-line RapidR program for the editor benchmarks |
| `tools/bench/ui/studio_caret.py` | Studio's caret moves and edit pauses in a big file (headless) |
| `tools/bench/sample_profile.py` | `sample` a command; read a report (inclusive, callees, self time) |
| `tools/bench/ui_bench`, `tools/bench/ui_budgets.json` (step 2) | the guard: the UI's and the editor's budgets |

## The pillars

- **Fast**: this is the list; the UI and editor fixes are caches, indexes
  and deferrals, measured before and after.
- **Easy to use**: no new API; programs get faster as they are.
- **Accessible**: a UI that answers in a frame keeps screen readers and
  keyboard users in step; nothing in the accessibility tree changes.
- **Beautiful**: same pixels; smooth resizing instead of stutter.
- **Legal**: no new dependency yet; RapidQ's include files were only read
  locally, never copied into the repository.

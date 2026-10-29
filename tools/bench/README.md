# The frame benchmark (tools/bench/)

How fast RapidR's data frame (`RDataFrame`, the engine in
`crates/rapidr-value/src/datascience/frame/`) is on a big table, measured
the same way every time, beside pandas and Polars when they are installed —
docs/data-science-plan.md §9.

## What it measures

One table, the same for every tool: **1,000,000 rows** (or any number),
five columns — `id`, `name`, `dept`, `salary`, `years` — 36.6 MB of CSV,
generated deterministically (a xorshift generator: the same file every
time), and a five-row second table (`depts.csv`: each department's floor)
for the join. On it:

| Step | What |
|---|---|
| load | CSV text → a frame (from memory: the text is already read; the RapidR program reads the file) |
| filter | `salary > 75000` (about half the rows) |
| sort | by `salary`, highest first, stably |
| group-by | by `dept` (text), the mean of the other columns (`id`, `salary`, `years`; `name` isn't a number) |
| join | a left join with `depts` on `dept` |
| describe | count, mean, std, min, quartiles, max of the numeric columns |
| save | the frame → CSV text (to memory; the RapidR program writes the file) |
| pipeline | load, filter, sort and group-by one after another on one frame |

Each step but the pipeline starts from a fresh copy of the loaded table;
each time is the median of 3 runs (`--runs N` to change it).

## The tools

| File | Runs |
|---|---|
| `frame_bench` (`crates/rapidr-value/examples/frame_bench.rs`) | the engine alone: `cargo run --release -p rapidr-value --example frame_bench -- [rows] [--runs N] [--threads N]`. Prints each step beside its **phase-1 budget** (load 120, filter 25, sort 120, group-by 60, join 60, describe 60, save 100 ms for a million rows; scaled with the rows) and **exits with 1 when a step misses its budget by more than 25 %**, or the pipeline takes a second or more (5 s for ten million rows). `--threads 1`: every step on one thread, as in a web page. The `digest` line fingerprints the answers: two engines that answer alike print the same digest. `--write-csv DIR` writes the table (`DIR/people_ROWS.csv`, `DIR/depts.csv`) and stops. |
| `bench_rapidr.rr` | the same steps as a RapidR program (`RDataFrame`'s members, timed with `TIMER`): `./rapidr run tools/bench/bench_rapidr.rr [rows]`, or built native with `./rapidr build`. Its times include the program around the engine. |
| `bench_pandas.py` | the same steps in pandas: `python3 tools/bench/bench_pandas.py [rows]`. |
| `bench_polars.py` | the same steps in Polars: `python3 tools/bench/bench_polars.py [rows]`. |
| `compare.sh` | all of the above that this machine has, then one table: `tools/bench/compare.sh [rows]`. |
| `systems_loops.rr` | the main loops of the systems recipes (memory helpers, a hex dump's rows, a file read and written, the tiny CPU's step loop), no window: `./rapidr run tools/bench/systems_loops.rr [memory\|hexdump\|filewrite\|fileread\|cpu\|cpu2]`, or built native; the numbers are on [the manual page](../../docs/manual/recipes-systems.md#how-fast). |

pandas and Polars are never dependencies of RapidR and nothing here
installs them: their scripts say so and stop cleanly when the library
isn't there. The data goes to `tests/results/bench/` (git-ignored); a
native build of the RapidR program compiles in the build cache
(`RAPIDR_BUILD_CACHE`).

## The numbers on the reference Mac

Apple M4 Max, 16 cores, macOS; release builds; 2026-10-15. The Mac was
shared with other work while these ran (load average 20–30), so the
many-thread times are on the slow side and vary by tens of percent from
run to run; the one-thread times are steadier.

**`tools/bench/compare.sh` — 1,000,000 rows** (`RUNS=9`; milliseconds):

| step | RapidR engine, 16 threads | RapidR engine, 1 thread (as the web) | RapidR program, `rapidr run` | RapidR program, native | pandas 3.0.1 | Polars |
|---|--:|--:|--:|--:|--:|--:|
| load | 11.8 | 69.9 | 19.9 | 21.7 | 287.6 | not installed |
| filter | 2.9 | 12.6 | 4.2 | 4.5 | 10.0 | not installed |
| sort | 10.2 | 51.6 | 11.9 | 14.8 | 149.2 | not installed |
| group-by | 4.8 | 19.0 | 4.3 | 4.2 | 22.1 | not installed |
| join | 3.4 | 20.1 | 3.3 | 3.3 | 50.4 | not installed |
| describe | 10.1 | 28.2 | 9.4 | 9.5 | 28.5 | not installed |
| save | 7.3 | 54.4 | 25.3 | 33.8 | 615.5 | not installed |
| **pipeline** | **22.7** | **116.2** | **24.5** | **25.3** | **371.6** | not installed |

**10,000,000 rows** (375.5 MB of CSV; `RUNS=3`):

| step | RapidR engine, 16 threads | RapidR engine, 1 thread (as the web) | RapidR program, `rapidr run` | RapidR program, native | pandas 3.0.1 | Polars |
|---|--:|--:|--:|--:|--:|--:|
| load | 133.9 | 765.1 | 202.5 | 176.4 | 3,162.1 | not installed |
| filter | 30.8 | 191.5 | 31.0 | 40.1 | 125.4 | not installed |
| sort | 141.7 | 1,094.8 | 181.3 | 190.1 | 4,718.6 | not installed |
| group-by | 40.7 | 190.6 | 48.2 | 42.8 | 212.4 | not installed |
| join | 31.7 | 199.3 | 52.3 | 51.3 | 577.2 | not installed |
| describe | 99.9 | 283.1 | 106.0 | 100.6 | 260.1 | not installed |
| save | 78.8 | 685.6 | 267.0 | 116.2 | 6,538.9 | not installed |
| **pipeline** | **273.1** | **1,571.7** | **283.5** | **358.1** | **5,792.9** | not installed |

The RapidR program's load and save include reading and writing the file
(the engine's columns work from text in memory), and each of its times is
one run; around the engine, the interpreter costs next to nothing — a
RapidR program is as fast as its frame. pandas reads the CSV from memory
(`BytesIO`) with its default engine. Polars isn't installed on this Mac
(`bench_polars.py` is ready for a machine that has it).

**Before and after** — the same harness on the engine of commit 920c8a85
(each column's text in one buffer, its numbers worked out once, every step
on one thread) and on the typed, parallel engine of 2026-10-15; the same
answers (frame_bench's digest: `0b1ad4ecf8bc9327` for a million rows,
`0aca0110893679c4` for ten million, both engines):

| step | 1 M before | 1 M after (16 threads) | 1 M after (1 thread) | 10 M before | 10 M after (16 threads) | 10 M after (1 thread) |
|---|--:|--:|--:|--:|--:|--:|
| load | 100.2 | 11.8 | 69.9 | 1,147.6 | 133.9 | 765.1 |
| filter | 39.3 | 2.9 | 12.6 | 421.0 | 30.8 | 191.5 |
| sort | 203.0 | 10.2 | 51.6 | 4,645.5 | 141.7 | 1,094.8 |
| group-by | 83.5 | 4.8 | 19.0 | 913.4 | 40.7 | 190.6 |
| join | 62.6 | 3.4 | 20.1 | 688.6 | 31.7 | 199.3 |
| describe | 85.5 | 10.1 | 28.2 | 915.8 | 99.9 | 283.1 |
| save | 82.3 | 7.3 | 54.4 | 835.5 | 78.8 | 685.6 |
| **pipeline** | **266.9** | **22.7** | **116.2** | **4,394.5** | **273.1** | **1,571.7** |

Every phase-1 budget of the plan is met with room to spare — on all cores
and on one — and the pipeline (load + filter + sort + group-by) takes 23
ms for a million rows (target 300 ms, hard limit 1 s) and 0.27 s for ten
million (limit 5 s); most steps also beat the plan's phase-3
"Polars-class" goals (load 40, filter 5, sort 30, group-by 15, join 20,
describe 10, save 40, pipeline 100 ms).

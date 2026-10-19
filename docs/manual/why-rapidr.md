# Why RapidR

RapidR is a modern home for BASIC programs: you write one source and it runs
on today's computers — Windows, macOS, Linux and the web — with a modern look,
accessibility, data tools and an AI that never holds your keys.

This page says what RapidR does best. **Every number on it was measured,
and the conditions are beside it**; the programs that measured them are in
the repository (`tools/bench/`), so you can run them on your own computer and
get your own numbers. Where something hasn't been measured, this page makes
no claim.

## Data: a table of a million rows in a blink

RapidR's `RDataFrame` is its own engine, written for RapidR in pure Rust:
columns of numbers stored as numbers, big jobs shared out to every core of the
computer. The same engine runs in native programs, in the interpreter and in a
web page, so an answer is the same everywhere.

Measured on one table everyone gets the same way: 1,000,000 rows, five columns
(a 36.6 MB CSV file), made by the same generator for every tool. Times in
milliseconds, the median of runs, on an Apple M4 Max (16 cores) with macOS,
release builds, 2026-10-15 — the Mac was busy with other work (load average 20
to 30), so these are, if anything, on the slow side. pandas is 3.0.1 reading the
CSV from memory with its default engine.

| Step on 1,000,000 rows | RapidR | pandas 3.0.1 |
|---|--:|--:|
| load (CSV text to a table) | 11.8 | 287.6 |
| filter (`salary > 75000`, half the rows) | 2.9 | 10.0 |
| sort (by salary, stably) | 10.2 | 149.2 |
| group by department, the mean of the numbers | 4.8 | 22.1 |
| join with a small table | 3.4 | 50.4 |
| describe (count, mean, std, quartiles …) | 10.1 | 28.5 |
| save (table to CSV text) | 7.3 | 615.5 |
| **load + filter + sort + group by** | **22.7** | **371.6** |

- **Ten million rows** (375 MB of CSV): the same four steps take **0.27 s**
  (273 ms) in RapidR; pandas takes 5.8 s on the same Mac.
- **A RapidR program is as fast as its engine**: the same four steps written in
  BASIC took 24.5 ms with `rapidr run` and 25.3 ms built native (a million
  rows, file reading included).
- **In a browser**, where a page gets one core: the same four steps take 116 ms
  for a million rows in the engine on one thread, and Chrome on the same Mac
  loads the million rows in about 0.15 s, filters them in 10 ms, sorts in 60 ms
  and groups in about 35 ms ([Data science with RapidR](data-science-with-rapidr.md#5-a-million-rows)).
- The engine answers exactly as the one before it did (checked against it
  over tens of thousands of random programs): faster, never different.

The benchmark, its table generator and the budgets a build must meet:
[tools/bench/README.md](../../tools/bench/README.md). How to use data in a
program: [Data science](data-science.md) and the step-by-step
[Data science with RapidR](data-science-with-rapidr.md), where a table from a
file shows live in the designer before you write a line of code.

## A lean, quick virtual machine

The RapidR Runtime — what runs your program when it isn't a native build — is
a bytecode virtual machine made to be quick, small and quiet.

| Measured 2026-10-15 | A Mac (Apple silicon) | Linux in a VM, one core, 2 GB |
|---|--:|--:|
| Start-up: compile `hello.rr` and run it (wall time) | 6.5 ms | 2.4 ms |
| A loop of 10 million iterations | 0.6 s | 1.29 s |
| …under the debugger, with a breakpoint set | 0.7 s | 1.33 s |
| Memory of a small program, and of that loop | 12 MB | — |
| A window waiting for events, under the debugger | 0 % CPU | 0 % CPU |
| RapidR Studio with that program running | 0.25 % of a core | 0.25 % of a core |

(Times are CPU time, the fastest of several runs, from `tools/bench/vm_bench`
(the Mac's loop: 0.61 s in the first run, 0.58 s when measured again for this
page), which also keeps the other budgets in `tools/bench/vm_budgets.json`; a
missed budget fails the release's checks. A program waiting for its events
doesn't poll: nothing wakes it until something happens, so a laptop's battery
and a Raspberry Pi's one core aren't spent on waiting.)

**The debugger is quick too.** A step — F10, until the new line and its
values are on the screen — took 14 ms in RapidR Studio on the desktop and 19 ms
in Studio in the browser on a quiet Mac (the recorded baseline of
`tests/studio_perf.mjs`, the median of three). Measured again for this page,
with other work running, it took 19 ms and 29 ms (a load average of 9), and
21 ms and 44 ms (a load average over 30); the budget is 50 ms. Studio itself
starts in 0.28 s on that Linux VM.

## One program, every system

You write a program once. It runs as a native executable, in the interpreter,
and in a web browser, on Windows, macOS and Linux — and **it is checked to behave
the same**:

- A **conformance suite of over 250 programs**, each with its expected output,
  runs on all three runtimes.
- RapidR draws every window itself (its UI kernel), so a program doesn't change
  with the system it runs on. The tests **capture every window of the GUI test
  programs on the desktop and in a browser and require the two pictures to
  match byte for byte, and their accessibility trees to be equal** (over a
  hundred cases, each at 1× and 2× screen scale). RapidR Studio's own windows
  match the same way.
- What users expect from their system stays the system's: the macOS menu bar,
  the Open and Save dialogs, the clipboard, the tray icon.
- A web build is a static `.zip` for any web host: no server program, nothing
  contacted unless the program does it ([The web](web.md)).

## Older programs run unchanged

RapidR also runs programs written for RapidQ. The rule: what those programs
do, they keep doing, and RapidR's extras are additive. A compatibility suite of
428 older example programs keeps watch, program by program, and the results
are published whole:

- **On Windows** (2026-10-15): every program of the suite is run, interpreted
  and native, and its windows and output compared with the suite's reference
  results. Of the 428, **140 come out the same**, 51 differ in some way
  the notes describe, 32 fail (mostly what a 64-bit program can't do: 32-bit
  DLLs, x86 machine code), 13 aren't comparable, 64 aren't run (they would
  print, write the registry, start programs …), and **128 are programs
  the original compiler doesn't build or run**.
  [Every program and what was seen](../rapidq-corpus-status.md).
- **On macOS, Linux and the web**: of the portable programs (the API the older
  Windows and Linux versions shared), 114 ran unattended on each of macOS and
  Linux, interpreted and native, with **no problem found**, and the web matches
  the desktop. [Every program](../rapidq-corpus-status-unix.md).
- 358 of the 428 compile; the rest mostly call Windows' own libraries, which
  RapidR calls only on Windows, or use OLE, which it doesn't
  ([Differences](differences.md#what-rapidr-doesnt-do)).

To move a program to RapidR's own component names, [import it](importing-rapidq.md):
a copy, proved to compile to the same bytecode, with a report.

## Accessible without trying

Every program is usable from the keyboard and by a screen reader, with nothing
for the author to do: each window publishes an accessibility tree (roles,
names, values, states, actions) to VoiceOver on macOS, Narrator and NVDA on
Windows and AT-SPI on Linux; in a browser, an ARIA mirror of the same tree.
Text contrast meets WCAG AA in every theme and AAA in the high-contrast one;
the look follows the system's light, dark or high-contrast setting. Rich text
reaches a screen reader with its formatting, and RapidR Studio itself is made
the same way ([Components and objects](components.md#accessibility)).

## Secure by design

- **Web builds carry a Content-Security-Policy** made from what the program
  uses: no inline scripts, other servers only for the network components and the
  addresses the program spells out ([Building apps](building-apps.md#build-a-web-app)).
- **A bad value is an error, not a crash**: memory functions (`VARPTR`, `PEEK`,
  `POKE`) work on views of the program's own data and refuse any other address;
  hostile values stop the program with a message and a line.
- **SQL parameters** are bound, never pasted: text from a user can't change a
  statement ([Databases](databases.md)).
- **No encryption code is compiled into your program**: HTTPS uses the
  operating system's own TLS and certificates.
- **A downloaded program asks before it runs**, once per file.
- The web is audited as hard as the desktop: [the security audit](../security-audit.md)
  and [the policy for reporting a problem](../../SECURITY.md).

## AI, with your keys never in your programs

RapidR Studio has an AI assistant that stays **off until you turn it on**, and
the `RAI` component gives your own programs the same AI — Anthropic (Claude),
OpenAI, DeepSeek or any OpenAI-compatible server, including a model on your own
computer that sends nothing out.

- The assistant only **proposes**: its changes are diffs you accept or reject,
  one undo step each, and a new program is previewed before anything is written.
- **A key is never written into a project, a build, a log or a screenshot**:
  it lives in your system's keychain, or is asked of the user when the program
  runs; a build that finds one in a program's source warns you.
- **You can see everything sent**: "What was sent" shows the last request, and
  every request and decision goes to an audit log on your computer. RapidR has
  no telemetry.
- It's cheap on purpose: a "Build me a hello-form program" request took about
  7,600 to 22,000 tokens sent (a third to two thirds from the provider's cache)
  with each provider's fast model, measured 2026-10-15 — a fraction of a cent.

[The AI assistant and RAI](ai-assistant.md).

## What this page doesn't claim

- The speed numbers are from one Mac (and one small Linux VM); your computer
  will differ, which is why the scripts are in the repository.
- A browser gives a page one core, so the data engine is slower there than on
  the desktop (numbers above).
- Calls into Windows' own libraries (`DECLARE … LIB "user32"`) work only on
  Windows; RapidR doesn't pretend to be Windows ([Differences](differences.md)).
- RapidR is a Preview. The roadmap lists what isn't there yet:
  [ROADMAP.md](../../ROADMAP.md).

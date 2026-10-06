# How RapidR is developed independently of RapidQ

A record of the process that keeps RapidR an independent implementation:
what is used to learn how RapidQ behaves, what is never used, how that is
checked, and what was corrected when it went wrong. It is evidence of
independent development, kept up to date with the project. The legal
analysis behind it is in [rapidq-review.md](rapidq-review.md).

## 1. What RapidR is built from

RapidR is written from scratch in Rust by its author, with AI coding
assistants under his direction. Its specification is RapidQ's **interface
and behaviour**, learned from four sources:

| Source | Used for | Kept where |
|---|---|---|
| RapidQ's manual (the `rapidq.chm` help, its online mirror at rapidq.phatcode.net) | the language and the components' names, properties, methods and events, and what they are documented to do | read locally (`.reference/`, never committed); RapidR's manual is written in its own words |
| RapidQ's compiler, `RC.EXE`, run as a black box | what programs actually print, return or refuse: number formatting, edge cases, error conditions | a locally installed copy, run in the developer's own Windows test VM (`tools/rc_probe.sh`); never in the repository or any RapidR build |
| RapidQ's example corpus and community programs | which features real programs use, and whether RapidR runs them | read and run locally; never copied into the repository |
| Public documentation of the platforms RapidQ wraps: Microsoft's Win32 and DirectX documentation, Embarcadero's documentation of Delphi's VCL types, published file formats | the numbers behind constants, file formats (`.X`, `.DXG`, BMP, AVI, MIDI, WAV) | cited in the code where used |

## 2. What is never used

- RapidQ's source code (it was never published) and anything decompiled or
  disassembled from `RC.EXE` or RapidQ's libraries. The only things read
  from the binary are text strings that are themselves interface facts: the
  compiler's short error messages and the names of its built-in objects and
  members.
- Text from RapidQ's manual, beyond a quote of a sentence or less where a
  document needs it, attributed.
- RapidQ's include files, templates, example programs, images, icons and
  other media.
- Code from RapidQ community libraries, whatever their licence (several are
  GPL, others state no terms): where RapidR offers the same component, it is
  RapidR's own implementation of the documented or observed behaviour.

The rule for contributors is in [CONTRIBUTING.md](../../CONTRIBUTING.md).

## 3. How behaviour is ported

1. A question comes up (how does `PRINT` format 1E15? what does
   `CONVBASE$` accept?).
2. The developer writes a small test program of RapidR's own to ask it.
3. The program runs under RapidR and, when RapidQ's manual doesn't settle
   it, under `RC.EXE` in the test VM (`tools/rc_probe.sh`,
   `tools/rapidq_truth.py`, described in
   [../rapidq-ground-truth.md](../rapidq-ground-truth.md)).
4. RapidR's implementation is written or corrected until the outputs match.
5. The test program, with RapidQ's output as its expected output, joins the
   conformance suite (`tests/conformance/cases`), which runs on all three of
   RapidR's runtimes.

The outputs of RapidQ's own example programs, which RapidR also compares,
stay on the developer's machine (`.reference/rapidq_golden/`): they are
other people's programs' output.

## 4. Audits and corrections

| Date | What | Result |
|---|---|---|
| October 2026 | Audit of the repository for RapidQ-derived material ([../licensing.md](../licensing.md) §6) | `QDirListView.inc` and `QDockForm.inc` (RapidR's versions of two community libraries) were found to follow the originals' method bodies too closely and were rewritten as RapidR's own code with the same interface; four conformance cases and one GUI fixture that reused the manual's examples were replaced by RapidR's own programs |
| 2026-10-06 | Git history rewrite | every earlier version of those files was replaced by the clean version in every commit (`git filter-repo`); copies cloned before that date and GitHub's cache of old commit hashes may still hold them |
| 2026-10-06 | Review of RapidQ's terms, rights and trademarks, and of everything RapidR takes from RapidQ ([rapidq-review.md](rapidq-review.md)) | the built-in `RAPIDQ.INC` constants were regrouped by public origin in RapidR's own arrangement (same names and numbers, pinned by a test); the corpus programs' outputs moved out of the repository; public wording tightened (LEGAL.md, README, release notes); this record and CONTRIBUTING.md added |

Add a row whenever material is found and corrected, or the process changes.

## 5. Checking it again

- **No RapidQ file in the repository**: compare every blob of the history
  with RapidQ's distribution by hash, and every text blob by shared runs of
  words (the method of the 2026-10-06 review, [rapidq-review.md](rapidq-review.md) §9).
- **No RapidQ material in builds**: the installers and bundles are made
  from the repository only (`tools/release/`); nothing reads `RAPIDQ_DIR`
  or `.reference/` there.
- **`.reference/` stays ignored**: `git check-ignore .reference/x` and
  `git log --all -- .reference` (empty).

# Contributing to RapidR

Thank you for helping. Bug reports, test programs, fixes and documentation
are all welcome: open an issue or a pull request at
<https://github.com/iBobX/RapidR>.

RapidR is MIT-licensed (see [LICENSE](LICENSE)). By contributing, you agree
that your contribution is licensed under the MIT License too, and you
confirm that you wrote it yourself or otherwise have the right to submit it
under that licence.

## The one rule that matters most: nothing of RapidQ's

RapidR is compatible with RapidQ, and it is an **independent
implementation**. Its value, and its safety for everyone who builds
commercial programs with it, depend on it containing nothing that belongs to
RapidQ's rights holders or to anyone else. So:

**Never put into RapidR, in any file, commit, issue or pull request:**

- code, include files (`RAPIDQ.INC`, `RAPIDQ2.INC`, `windows.inc`, `qcgi.inc`
  and the other `.inc` libraries), templates or example programs from
  RapidQ's distribution, from RapidQ community sites, or from any program
  whose licence you don't know;
- text from RapidQ's manual or help files (`rapidq.chm`, the online mirrors),
  beyond a quote of a sentence or less where a test or a document really
  needs it, in quotation marks and attributed;
- RapidQ's images, icons, bitmaps, sounds or other media;
- RapidQ's compiler (`RC.EXE`), libraries, DLLs or any other binary, or
  output of decompiling or disassembling them.

**What you may use**, because it is the interface programs are written
against and compatibility needs it:

- the language's syntax and keywords;
- the names of components, properties, methods, events, functions and
  constants, and the constants' numbers (written in RapidR's own order and
  layout, with the public origin of each number noted where there is one:
  Microsoft's Win32 documentation, Embarcadero's documentation of the VCL
  types);
- behaviour: what a program prints, returns or shows, observed by running
  **your own** test programs (with RapidR, or with RapidQ as described in
  [docs/rapidq-ground-truth.md](docs/rapidq-ground-truth.md));
- file formats programs read or write;
- RapidQ's short compiler messages where RapidR reproduces a message for
  compatibility (a handful of words each).

Describe all of it **in your own words**. Write test programs yourself; don't
adapt the manual's examples or programs from the RapidQ corpus. When a
behaviour is ported from observation, say so in the commit or the code
comment ("as RC.EXE prints", "observed with RapidQ").

The same applies to other products RapidQ used or imitated (Delphi's VCL,
Windows, DelphiX, DirectX): names, numbers, formats and behaviour, never
their code, documentation text or artwork.

## Where RapidQ material may live on your machine

To study or test RapidQ, keep its distribution outside the repository (for
example `~/Downloads/Rapidq`, the default of `tools/rc_probe.sh`), and
downloaded documentation only in `.reference/`, which is ignored by git and
never committed. `RC.EXE` runs in your own test machine; RapidR's tools
copy programs to it and read their output, and nothing of RapidQ's is ever
written into the repository or into a RapidR build or installer.

## Names and trademarks

Use "RapidQ" (and Delphi, Windows, DirectX, …) only to say what RapidR is
compatible with or to name a platform ("compatible with RapidQ", "as in
RapidQ"). Never name a RapidR component, package, file, theme or product
after them, and never use their logos. RapidR's own components are `R…`;
the `Q…` names exist only so that RapidQ programs run.

## Checks before a pull request

- `tools/regress.sh` (or the stages your change touches), as in
  [README.md](README.md);
- for any new dependency: a permissive licence accepted by `deny.toml`
  (`tools/regress.sh legal`);
- for anything ported from RapidQ's behaviour: a conformance case you wrote,
  with RapidQ's output as its `.expected` when you could run RC.EXE.

Questions about whether something is safe to add: ask in the issue or pull
request before adding it. The background is in
[docs/legal/rapidq-review.md](docs/legal/rapidq-review.md) and
[docs/legal/clean-room.md](docs/legal/clean-room.md).

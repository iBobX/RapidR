## How it was run

- **Programs**: RapidQ's examples (`~/Downloads/Rapidq/examples`, never in this repository), the
  ones `tools/rapidq_corpus.py` classes as *portable* (only the API RapidQ's Windows and Linux
  versions shared: no DLL routines of their own, no DirectX, OLE or port I/O). Each runs from a
  copy of its folder.
- **macOS** (27, Apple silicon) and **Linux** (Ubuntu 26.04 ARM64 in the Parallels VM, its
  case-sensitive ext4): `tools/corpus_run.py --list` — interpreted (`rapidr run`) and native
  (`rapidr build`, release; universal on macOS), GUI programs on the headless host with their
  windows captured after 2 s (classic look, as RC.EXE's), message boxes and file dialogs
  answered, console programs with no input and again in a terminal (keys typed). Nothing prints
  on paper (`RAPIDR_PRINT_TO`), QREGISTRY writes a scratch store; programs that use the network,
  a serial port, print or start other programs are *not run* (the printing ones were checked
  apart: their documents land as PDFs).
- **Web**: `tests/corpus_web_compare.mjs` — the program on the web runtime (Chromium, the wasm
  compiler and VM) against the desktop interpreter: what it prints, and every form's and
  component's place, size and visibility as the program reads them.
- **RC.EXE**: console programs whose RapidQ output is known (`tools/rapidq_truth.py`) are
  compared byte for byte (10 portable ones that don't print the clock or random numbers: none differs, on any system or backend); behaviour in doubt was settled with probes run by
  RC.EXE in the Windows VM (`tools/rc_probe.sh -w`, `w_*` window probes).
- macOS and Linux captures were compared pixel for pixel: identical (fonts and metrics are
  RapidR's own on both) except where a program's menu is in the window (Linux, Windows and the
  web) or in macOS' menu bar, a program's random numbers or clock, and what the file system holds
  (QDIRTREE).
- **ok for a GUI program** means its windows showed, not one colour, with no error or warning;
  *how* they look was checked by eye (contact sheets of every capture; two programs drew wrongly
  and are fixed below) and, for a few, in real windows on Ubuntu's desktop (Wayland). Side by
  side with RC.EXE's own windows is the Windows lane's (docs/rapidq-corpus-status.md).

## What this pass fixed

| Cause | Programs | Fix |
|---|---|---|
| Windows file names: `\` was part of a name off Windows, and Linux (and a case-sensitive macOS volume) told `OPEN.BMP` from `open.bmp` | Linux: `listbox/owndraw`, `games/puzzle`, `Fonts/VECTOR_Font`, `graphics/vfont`, six DirectX examples' models and textures; any program using `Application.Path + "\file"` | `\` separates folders, names found in any case, as on Windows and the web (`rapidr_value::paths`; case `rapidq_windows_paths`) |
| A `lbOwnerDrawVariable` list measured its items, then waited for the mouse to move before OnDrawItem drew them (interpreted) | `listbox/owndraw` (every system) | the drawing handlers run in the same turn (GUI case `owner_list_variable`) |
| A TYPE's instance CREATEd inside a form's CREATE was made after the whole block — after its `ShowModal` returned | `richedit/syntax` (an empty window) | made where it stands, as RC.EXE (GUI case `nested_typed_create`) |
| QRICHEDIT's SelStart / SelLength counted a line break as one character, its Text has two (CR LF): RC.EXE counts two | `richedit/syntax` (looped forever) | positions in Text, as RC.EXE (`textedit` tests, `text_edits`) |
| RapidR's built-in RAPIDQ.INC had no `QBColor` array (QBasic's 16 colours) | `sound/SBBAS` (a hardware program), `mysql/SQLServerTutorial/Taxes` (network) | `QBColor(0 TO 15) AS INTEGER` as RC.EXE reads it (case `rapidq_qbcolor`) |
| The sweep tools: `x.bas` and `x.rqb` shared a work folder; includes found case-sensitively on Linux (programs classed differently there); quarantined read-only copies refused; native executables looked for in the wrong folder; a program's own modal form taken for a message box on the web side; several `$RESOURCE`s on a line lost on the web side | — | `tools/corpus_run.py`, `rapidq_corpus.py`, `rapidq_truth.py`, `tests/corpus_web_compare.mjs` |

## What's left

- **Windows calls made by an include** (the list above): RapidQ's `rapidq2.inc` calls kernel32's
  `GetVersion` as it starts (its SoundEx), so every program including it stops at once off
  Windows; with that call out of the way the eight tried stop at the next Windows call
  (GetProcessHeap, SetWindowLong, AlphaBlend, OpenGL), but for `graphics/jpeg_Blend` and
  `graphics/image_test`. Answering `GetVersion` off Windows would be emulating Windows: a
  decision for Robert (the DLL lane's).
- ~~**Rich text in QRICHEDIT**~~ (`SelAttributes`, bold and coloured runs): done (RICHEDIT,
  2026-10-15, docs/manual/rich-text.md): `richedit/syntax` shows its highlighting, its saved RTF
  byte-identical to RC.EXE's.
- **Form frames and menus** (not Unix-specific): RapidR's frame is Windows 11's total (a 1-pixel
  border and a 29-pixel caption; an in-window menu 28 pixels), so a program sized for an older
  Windows (Othello, the sliding puzzle, multilevelUndo) shows scroll bars where its author's
  window fitted — as RC.EXE's own build does on Windows 11. On macOS the menu is in the system's
  menu bar and the window keeps its height, so the same programs fit there.
- **RC.EXE's own failures**: the programs marked ok\* behave as RC.EXE's builds do.
- **Not run here**: programs using the network or a serial port; the Windows lane's sweep covers
  RC.EXE's captures side by side (docs/rapidq-corpus-status.md).

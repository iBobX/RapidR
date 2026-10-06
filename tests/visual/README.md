# The visual gallery

What RapidR's forms look like, checked against approved images — and
against RapidQ itself. The other GUI tests compare RapidR with itself (the
web against the desktop, a few pixel colours); this one is about *looking
right*.

- `cases/` — small RapidQ programs, each showing components in their
  typical states (buttons default, focused, disabled, held down; check boxes
  and radio buttons on, off, disabled; panels' bevels; lists, trees, grids,
  tabs, track bars, scroll bars, gauges, up-downs, menus, status bars,
  message boxes, fonts …). They are plain RapidQ: RapidQ's own compiler
  builds them. A `<name>.rapidq.bas` beside one is the same program as
  RapidQ needs it written (`updown`: RapidQ's QUPDOWN is an include library
  over Windows' up-down control, made directly there). Comment lines in a
  case: `' press: <caption>` holds that button down in RapidQ's run
  (BM_SETSTATE), `' rapidr-env: NAME=value …` sets RapidR's test hooks for
  its run (`RAPIDR_TEST_EVENTS`, `RAPIDR_TEST_MESSAGE_DIALOG` …).
- `rapidq/` — RapidQ's own windows: each case built by RC.EXE (Rapid-Q
  2006) and run in the Windows 11 VM, unthemed (RapidQ's programs have no
  comctl6 manifest: Windows' classic look, RapidR's `classic` theme),
  captured at 1× (96 dpi, as drawn) and at 2× (the program run with Windows'
  GDI scaling on the VM's 200 % screen: GDI draws its lines and text at the
  screen's resolution — the crispness RapidR's 2× is held to).
  `tools/visual/rq_shots.sh [case,…]` makes them (docs/rapidq-ground-truth.md
  has the VM's set-up). Keyboard cues are shown (focus rectangles, `&`
  underlines), as RapidR always shows them.
- `golden/<theme>/` — RapidR's approved images: classic at 1× and 2×,
  modern, dark and high contrast at 1×.
- `out/` (not kept) — the last captures, a diff image beside each that
  differs from its golden, and `index.html`: the contact sheet, RapidQ's
  window beside RapidR's in every theme at 1× and 2×.

```sh
tools/visual/gallery.py check            # capture everything, compare (exit 1 on a difference)
tools/visual/gallery.py check buttons    # one case
tools/visual/gallery.py approve buttons  # a deliberate change: make the captures the goldens
tools/visual/gallery.py approve --all
open tests/visual/out/index.html
```

`tools/regress.sh visual` runs the check. Besides `tests/visual/cases`, the
gallery captures a few fixtures (the accessibility form, the themes form)
and the GUI examples (`examples/gui`, the canvas, Notepad).

A difference is a change in what a user sees: look at the sheet, and
approve it only when the new image is the right one (closer to RapidQ, or
a deliberate change of RapidR's own look) — never to make the check pass.

Deliberate differences from RapidQ's windows, for the record: text is
anti-aliased (RapidQ's MS Sans Serif is a bitmap font) though as wide and
as high as RapidQ's (RapidR Sans: `crates/rapidr-value/fonts/README.md`);
the window scroll bars of lists and memos are drawn classic, where Windows
11 draws its own thin ones even for unthemed programs; a form's caption and
borders are the platform's.

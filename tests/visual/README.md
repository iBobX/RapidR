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

## RapidR Studio's own goldens

The gallery above checks programs. This part checks **RapidR Studio itself**:
a change anywhere in what Studio draws — a pin that stops being drawn, a
pane that loses its label, a colour that moves, a menu that grows — fails
the test until a person has looked at the new picture and approved it. (The
other Studio tests compare the desktop with the web, so a feature that
vanished from both would pass; and the contrast gate only judges text.)

- `tools/visual/studio_scenes.py` — the scenes: Welcome; the designer on
  live data with a component selected (anchor pins) and after a drag; the
  code editor with the completion list, with a hover, with a squiggle and
  Problems; the debugger paused on `examples/debugging/shop.rr` (Variables,
  Call Stack, Watch with a changed value, Breakpoints, an expanded data tip,
  a run-time error); the AI panel and Settings > AI (against the mock
  provider, a made-up key); a Markdown preview; the rich-text example; a
  RapidQ import; the save prompt; Project Options; Find in Files; the
  command palette. `tools/visual/gallery.py studio list` names them and what
  each guards. Each is Studio started as `tests/studio_shell.mjs` and
  `tests/studio_flows.mjs` start it (`--do` steps, the test hooks).
- `studio/<look>/<scene>@<s>x.png` — the approved images, in RapidR light,
  dark and high contrast at 1×, and at 2× for Welcome, the designer, the
  completion list and the data tip (about 12 MB in all, 75 images). The
  desktop's captures are the ones kept: the web build's are byte-identical
  to them (`tests/studio_shell.mjs`).
- `out/studio/` (not kept) — the last captures, a contact sheet
  (`index.html`) and, beside each scene that differs, `…-compare.png`
  (approved | now | what changed), `…-zoom.png` (the changed part, enlarged)
  and `…-diff.png`.

```sh
tools/visual/gallery.py studio check                  # every scene, every look (exit 1 on a difference)
tools/visual/gallery.py studio check debug-watch      # one scene
tools/visual/gallery.py studio check --theme dark     # one look (light, dark, contrast), --scale 2
tools/visual/gallery.py studio approve debug-watch    # a deliberate change: the last captures become the approved images
tools/visual/gallery.py studio approve --all
tools/regress.sh studio-goldens                       # the same, as the regression stage (part of `studio`)
open tests/visual/out/studio/index.html
```

When it fails, open the `…-compare.png` the output names (and the
enlarged `…-zoom.png`): if the new picture is what Studio should look like
now (a feature was added, a look was changed on purpose), run `studio
approve <scene>` and commit the new images with the change; if it is not,
the change broke something — fix the change, never the image. Approving a
scene approves it in every look: look at all three (dark and high contrast
are where text goes unreadable).

**Why it can be trusted to stay still.** The renderer is deterministic to the
byte (the CPU renderer's portable SIMD level), so nothing is forgiven by
default (`RAPIDR_STUDIO_TOLERANCE=n` forgives n levels of a colour channel,
for a platform that rounds differently). What could vary is fixed by the
scene runner: the fonts are the ones RapidR carries; the window's size and
the menu (in the window, `RAPIDR_MENU=window`) are the same everywhere; every
run starts with `--fresh` (the default layout and settings) and its own
registry file; the AI keys are made up and kept in memory, and the AI scenes'
mock provider listens on port 18695 so its address is the same on the
screen — each run starts its own; when another run (another lane's) has
that port, this one's mock takes a free port, the scenes' Base URL follows
it, and the two AI scenes are run and kept as `…-other-port.png` but not
compared (their screen shows the other address), said in the output; scenes that write beside what they open work on a copy in a
folder of their own; no machine's own path may be on a scene's screen (a
scene that would show one — an import's report — doesn't: its Output is
covered and the report is not the document open); the Project Options
dialog is shown as on a machine without Rust. The check was run from a
different folder, under another locale and time zone, and with twelve scenes
at once: the same pictures.

A scene's script is timed by the clock (one step each 400 ms), so a machine
that is very busy can make a step late: a capture that differs is taken
again, up to twice, and passes if a retake matches — printed as `!` with the
scene's name, so a scene that needs it gets more delay or waits. Runs go
four at a time (`--jobs N`, `RAPIDR_STUDIO_JOBS`). A failure always names
its images; an approved image no scene makes any more fails a full check
(remove it).

**A new scene** (a feature that is a new thing to look at) is an entry in
`tools/visual/studio_scenes.py` — its `open`, its steps, its `delay` — then
`studio capture <name>`, look at the three looks in `out/studio/`, `studio
approve <name>`, `studio check <name>` a few times; keep the whole set under
about 25 MB (1× unless the scene is about crispness).

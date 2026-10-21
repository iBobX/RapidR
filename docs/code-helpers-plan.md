# Helpers: the code that shows and edits its own values (plan, 2026-10-21, part of Preview 4)

Robert (2026-10-21): when you write a colour, the code editor should show it, and one click should open a picker that edits the code. The same goes for files, images, fonts, sounds, choices and links to other components. A short help line should appear while you type. Helpers make people faster, and they make the AI faster too.

## 1. The idea
A **helper** is a small view of ordinary text. The code stays plain RapidR: readable anywhere, diffable, and written by people or AI as usual. The helper only shows what the value *is* and offers a better way to change it.

```
  Button1.Color = RGB(255, 130, 20) ■        ← a mark: the colour, from the code itself
  PlaySound("click.wav") ▶                    ← a mark: listen
  DrawCircle(x: 40, y: 40, r: 12)            ← chips: argument names
  Img.LoadFromFile "logo.png" 🖼  Loads a picture from a file   ← a line note while the caret is here
```

Click a mark, or press **Ctrl+.** (**Cmd+.**) on it, and a **value editor** opens. It might be a colour wheel, a font list or the project's pictures. What you choose is written back into the code as one undo step, in the same form you used (`RGB(…)`, `&H…`, or a colour constant).

**One value editor, everywhere.** The same colour editor serves:
- the code;
- the Properties panel;
- the designer;
- RAI's review cards, which show the old colour beside the new one;
- your programs at run time, through `RValueEditor`.

It is one implementation in many places: one look and one set of keys to learn.

## 2. What helpers are built on
1. **A `kind` for every value in the registry.** Today the registry knows a parameter's type, and a colour property's unit (`bgr`). We add one facet, `kind`, to properties, parameters and builtins:
   - `color`, `font`;
   - `file:image`, `file:sound`, `file:any`;
   - `shortcut`, `cursor`;
   - `choice:<set>` (alignments, cursors, message buttons as a flag set);
   - `range` (when min..max are known);
   - `component:<type>` (a link to another component, such as `DataSource` or `Parent`);
   - `handler` (an event's SUB).

   The AI reads the same facet, so RAI writes valid values (K1, the language card).
2. **Sites from the language service, never text matching.** The language service already holds the program's semantic model. It reports **helper sites**: a span, a kind, the parsed value, and how to write it back. Nested calls work, and so do constants and expressions it can evaluate. When it can't evaluate something, it shows nothing rather than a guess.
3. **Four ways to show, in the kernel's code editor** (desktop and web alike):
   - **Marks:** small and fixed-width; never move a line, and wrapping counts them. They show a swatch, a thumbnail, a font sample, ▶, a unit (`500 ms`, `45°`), or a red mark when a file is missing.
   - **Chips:** argument names before literal arguments. Settings: Off / While holding Ctrl / Always.
   - **The line note:** a one-line, dim help text at the end of the caret's line, from the registry's doc. Only on the caret's line, so there is no clutter.
   - **Values while running:** the current values at the end of the line while the program runs or is paused in the debugger.
4. **Value editors**, opened from a mark or with Ctrl+. (Cmd+.). They are popovers drawn by the kernel, write back as one undo step, and keep your format.

## 3. The helpers
★ = must-have for Preview 4; the rest board the release if ready (the train rule).

| # | Helper | What it does |
|---|---|---|
| H1 ★ | **Colour** | A swatch for `RGB()`, `&H…` literals and colour constants. The editor: a wheel with a square, RGB and hex fields, the colour's name, and the project's own colours as a palette. It keeps your format |
| H2 ★ | **Argument names** | Chips before literal arguments (all arguments while holding Ctrl) |
| H3 ★ | **Line note** | The call's one-line help at the end of the caret's line |
| H4 ★ | **Values while running** | Variables' values at the line's end while running or paused |
| H5 ★ | **Files and pictures** | A thumbnail, and a red mark when the file is missing. The editor: the project's files, filtered by kind, with a preview. Dropping a file onto the code writes its path |
| H6 ★ | **Fonts** | A sample in the font itself. The editor: name, size and style, with a live preview |
| H7 | **Sounds** | ▶ to listen (volume-limited) |
| H8 ★ | **Choices** | A dropdown for constant sets; tick boxes for flag sets (message buttons); a toggle for True/False |
| H9 | **Numbers** | Units shown. A slider when the registry knows the range. Alt+drag to scrub, with a dead zone; a typed field is always there too |
| H10 ★ | **Links** | `Parent =`, `DataSource =` and `OnClick =` offer the compatible components or SUBs, and **Create handler** |
| H11 | **Shortcuts** | Press the keys to record a shortcut |
| H12 | **Positions** | A mini preview of the rectangle on its form; "Show in designer" |
| H13 | **Patterns** | Test an RRegex pattern (S4) on sample text, with the matches highlighted |
| H14 | **Formats** | A live sample of a `FORMAT$` or date format |
| H15 | **Live tweak** (the WOW) | While the program runs from Studio, changing a colour or number through a helper changes the running program at once (on the VM, desktop and web). **Keep** writes it to the code; **Undo** puts it back |
| H16 | **Your own helpers** | `RValueEditor`: write a value editor in RapidR (`Kind`, `OnPaint`, `OnEdit`) and the code editor, the Properties panel and the designer use it. Components ship their own, such as curve and gradient editors (vmx-components-plan W4). Programs can also open the built-in colour and font editors at run time |

## 4. Our standards for helpers
**Easy**
- Helpers are on by default at the **Helpful** level: marks, the line note, and chips for literals.
- **Quiet** shows marks only; **Everything** shows all. Each helper also has its own switch.
- A helper never surprises you: nothing changes the code unless you choose something in an editor.

**Fast**
- Only visible lines are decorated, with a cap per screen.
- Nothing is recomputed while you type; the work waits for a short pause.
- Widths stay fixed on the line you are editing, so nothing flickers.
- Thumbnails are cached.
- Typing latency is measured on the 4,500-line project (S7).

**Accessible**
- Everything works from the keyboard:
  - **Ctrl+.** (Cmd+.) opens the helper at the caret;
  - Tab moves inside an editor and Esc closes it;
  - sliders take arrows, Home, End and Page Up/Down.
- Screen readers:
  - "Read line with helpers" speaks the marks ("orange, 255 130 20");
  - nothing is narrated live, because a short sound marks a helper instead.
- Colour is never the only cue: the editor shows the numbers and the name.
- Popovers stay while hovered and close with Esc, with targets of at least 24 px.
- Live tweak respects reduced motion.

**Secure**
- Thumbnails and sounds come only from inside the project folder.
- Nothing is fetched from the network.
- Patterns run on the linear-time regex engine.
- Write-backs are ordinary edits, so they can be undone and are recorded in Versions.

**Parity:** the code editor is a kernel component, so desktop and web behave the same. On the web, the file editor lists the project's files.

**Licences:** our own design and code. Ideas from public research only, and no code from copyleft tools.

## 5. Lanes
| Lane | Work | Model, size |
|---|---|---|
| E0 ★ | The framework. The `kind` facet in the registry, with a test that every colour, file and font value has one. Helper sites in the language service. Marks, chips and line notes in the kernel's code editor (no layout shift, wrap-aware, defined caret stops). The popover host; Ctrl+.; accessibility; settings | Opus L |
| E1 ★ | The value editors, shared with the Properties panel: colour, files and pictures, fonts, choices, links | Opus M + Sonnet M (visual, one lane at a time) |
| E2 ★ | H2–H4: argument names, the line note, values while running (from the signature, hover and debugger data we already have) | Sonnet M |
| E3 | H7, H9, H11–H14 | Sonnet M |
| E4 | H15 live tweak (VM literal patching, web too) | Opus M |
| E5 | H16 `RValueEditor` for programs and components | Opus M |
| E6 ★ | Filling `kind` across the registry (mechanical, checked by E0's test) | Sonnet S |

**Budget:** about 10–15% of a week. The ★ lanes come after Preview 4's quick wins, alongside S3 and S5.

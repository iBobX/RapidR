# RapidR's icons

RapidR's own icon set ([docs/ide-plan.md](../../docs/ide-plan.md), decision D8): every action, component, file type, symbol, toolbox group and glyph of RapidR Studio, also used by the manual, the website and the VS Code extension. Every icon is our own drawing, under the project's licence (MIT). There are 344 icons in six categories, each hinted at 16, 24 and 32 px.

| Category | What | Colour |
|---|---|---|
| `actions` | The IDE's commands (files, edit, run, debug, designer, docking, general) and the gutter's and problems list's markers | Monochrome (`currentColor`); colour only where it means something (run, stop, breakpoints, statuses, AI) |
| `glyphs` | Small marks inside controls: chevrons, carets, check, sort arrows, grips | Monochrome |
| `components` | One per component type (QBUTTON and RBUTTON share one), the IDE plan's planned components, RPLOT's chart kinds | Two-tone (neutral and one hue) |
| `files` | File types and folders | Two-tone |
| `symbols` | The language service's symbol and completion kinds | Two-tone |
| `groups` | The toolbox's groups | Two-tone |

The review sheets in [`review/`](review) show every category at 16, 24 and 32 px in the four themes at 1× and 2×. The mock-ups in `review/mockup-*.png` show the icons in context: a toolbar and a toolbox drawn by the UI kernel.

## The grid

- **The master is 24 × 24 px, with 2 px of padding.** The live area is 2…22.
- **Keylines** (master pixels):
  - a square 3…21;
  - a circle of radius 9.5 round (12, 12);
  - a portrait page 5…19 × 2…22 (the brand's file shape);
  - a landscape window 3…21 × 4…20.

  Shapes fill their keyline, so a square icon and a round one look the same size.
- **Each icon is drawn once, on the master.** The kit ([`tools/kit.py`](tools/kit.py)) hints it for each size: it maps the master's live area onto the size's own, then snaps every coordinate to that size's pixel grid.

| Size | Live area | Scale | Stroke | Stroke centres | Use |
|---|---|---|---|---|---|
| 16 | 1…15 | 0.7 | 1 px | n + 0.5: crisp | Toolbars, menus, trees, tabs, the gutter, at 1× |
| 24 | 2…22 | 1 | 1.5 px | n + 0.75: one edge crisp, crisp at 2× | 16 px icons at 1.5×; large toolbars; the manual |
| 32 | 2…30 | 1.4 | 2 px | n: crisp | 16 px icons at 2×; previews; the toolbox's large mode |

  - The stroke is always a sixteenth of the size, and a fourteenth of the live area, so the three sizes have one weight.
  - At 16 px the live area is relatively larger (1 px of padding), as small icons need.
  - Fill-only shapes snap their edges to whole pixels.
  - Circles keep their edges on the grid: the centre moves, the radius stays whole or half.
  - A tie in snapping goes away from the centre, so mirror-symmetric icons stay symmetric.
- **Renderers pick the drawing for the device size**, not the logical one: a 16 px icon at 1.5× is the 24 px drawing, and at 2× the 32 px one. A size with no drawing of its own gets a whole multiple of one (48 = 24 × 2), else the one nearest to a whole multiple ([`rapidr_icons::variant_for`](../../crates/rapidr-icons/src/lib.rs)).
- **Simplify at 16 px.** An icon may branch on the size, to drop a detail (the bug's third pair of legs, a scroll bar's arrows, a gear's two teeth) or to redraw a part. Detail never shrinks below what the size can show.
- **Spacing.** Parallel strokes stay at least 3 master px apart, centre to centre (a 1 px gap at 16 px). A tiny box at 16 px keeps a straight run on each side (its corner radius is at most a quarter of its shorter side).

## Strokes, corners, overlaps

- **Strokes** are 1.5 px on the master, with round caps and round joins: friendly at small sizes, and without the spikes of mitred joins.
- **Corner radius**: 2 px for frames (windows, fields, pages' bodies), 1.5 for small boxes, 1 for tiny ones, 3 for buttons and check boxes, 4.5 for the RapidR tile.
- **One weight.** No hairlines and no second stroke width. Fills carry the hierarchy: a tint for an area, a solid for an accent.
- **Overlaps are cut, not stacked.** A shape that sits on another (a badge, the front sheet of Copy, the plus of New file) cuts a gap of one stroke round itself out of what's behind it (an SVG mask, `G.cut`). So the icon works on any background, and nothing is painted in a background colour.
- **Perspective.** Icons are flat and frontal. The only exceptions are true 3D objects, which are drawn isometric at 30° (Direct3D's cube, mesh and frame, the module symbol). There are no shadows, gradients or glows, except the brand's blue-to-cyan gradient on the RapidR mark and on AI.

## Colour

Icons use **colour tokens**, never fixed colours. A token's value depends on the theme: `classic`, `modern`, `dark` and `highcontrast` (`rapidr_value::theme`). The SVG sources spell each token as its light-theme value, so a source is a valid, good-looking SVG on its own; the renderers swap in the theme's values. The palettes are in [`tools/palette.py`](tools/palette.py).

| Token | Light | Dark | Use |
|---|---|---|---|
| `currentColor` | `#5B6478` (the brand's Slate) | `#C9D1E3` (Mist) | Monochrome icons. The caller may set it (a selected row's text colour, for instance) |
| `ink` | `#5B6478` | `#C9D1E3` | Two-tone icons' neutral outlines |
| `paper` | `#FFFFFF` | `#373A42` | Neutral fills (a window's body, a page) |
| `shade` | `#E6EAF2` | `#4A4F5A` | A neutral tint (a disabled breakpoint, a keyword) |
| `blue` / `blue-tint` | `#2F5BFF` / `#E3E9FF` | `#7398FF` / `#24315C` | Controls, forms, the IDE's components (RapidR Blue) |
| `cyan` / `cyan-tint` | `#007F9C` / `#D7F3F9` | `#2CC9E9` / `#123C47` | Dialogs, network, the web |
| `teal` / `teal-tint` | `#08805E` / `#D6F2E8` | `#34CF98` / `#123D30` | Data; run, continue, success |
| `amber` / `amber-tint` | `#B86E00` / `#FFEDC7` | `#FFB224` / `#45340F` | Time, values, chart lines; warnings, events |
| `amber-solid` | `#FFB224` | `#FFB224` | Run Amber as a fill (the execution point, `.rrbc`'s counter) |
| `red` / `red-tint` | `#D92D36` / `#FCE1E2` | `#FF6B70` / `#4D1E22` | Stop, errors, breakpoints, the X axis |
| `violet` / `violet-tint` | `#6F42E5` / `#EBE4FD` | `#AC8FFF` / `#33295E` | Code you call (SUBs, functions), media, DirectX and Direct3D |
| `spark` | `#19C6E6` | `#19C6E6` | The brand gradient's end (blue to spark): the RapidR mark, AI |

- **Two-tone.** A component icon draws the control's shape as a neutral frame (`ink` on `paper`) and marks its key part in one hue, an outline over that hue's tint: the check of a check box, the arrow of a combo box, the selected row of a list. An icon has one hue, sometimes two when the meaning needs them (the X, Y and Z axes, a colour dialog).
- **Monochrome icons** (actions, glyphs) use `currentColor` only, so they follow the theme and the caller. Colour appears only where it carries meaning: run and continue (teal), stop and errors (red), warnings (amber), information (blue), hints (cyan), breakpoints (red), the execution point (Run Amber) and AI (the gradient).
- **Contrast.** In every theme, every hue's outline has at least 3:1 (WCAG 1.4.11, graphics) against the theme's backgrounds, against `paper` and against its own tint. `currentColor` has at least 4.5:1. The crate's tests check this (`contrast_in_every_theme`).
- **High contrast is monochrome.** Outlines and solids are white and tints are black, on Windows' High Contrast Black. Shape carries the meaning; nothing depends on hue.
- **Disabled.** Outlines and solids take the theme's disabled ink, and tints become `paper`.

## Metaphors

- **Shapes recur.** The page (with the brand's folded corner) is a file, the window is a form, the cylinder is a database, the magnifier is find, the cube is code you call, the four-pointed sparkle is AI. Components are drawn as miniatures of the control itself, not as symbols for it.
- **Badges.** A small mark in the bottom-right corner, cut out of the base, adds a meaning to another icon:
  - plus: new;
  - the database: a data-aware control;
  - the network: MySQL;
  - the magnifier: a query, a preview.
- **The brand's run triangle** keeps the proportions of the R's counter (wider than tall). It is Run, and it lights up `.rrbc` files and the Runtime.
- **The brand's R** (design/brand) marks RapidR's own files and the RapidR toolbox group, and is never redrawn.
- **No text.** Icons contain no words or digits. A letter shape is used only where the letter is the metaphor (the A of a label or a font), and it is drawn as geometry.
- **Nothing borrowed.** No icon is traced or adapted from another set (including RapidQ's bitmaps) or from a product's logo.

## Names

- An icon's **id** is `category/name`, in lower-case kebab-case: `actions/save-all`, `components/button`, `files/rr`. The sources are `src/<category>/<name>.svg` (24 px), `<name>-16.svg` and `<name>-32.svg`.
- **Actions** are named for what they do (`step-over`, `bring-to-front`).
- **Components** are named for their type without the R (`RBUTTON` → `button`, `RDBGRID` → `dbgrid`).
- **Files** are named for their kind (`rr`, `image`, `data`).
- **Lookups** (`rapidr_icons::get`) take an id, a bare name (searched in the order actions, glyphs, components, files, symbols, groups), a component type (`QBUTTON`, `RBUTTON`, `QGAUGE`) or a command id (`file.save`).

## Inventory

[`inventory.toml`](inventory.toml) says what has an icon:

- **Components** come from the code: every type in `rapidr_ast::COMPONENT_TYPES` gets the icon of its name, with exceptions in `[component-aliases]`. The IDE plan's components are listed in `[planned-components]` so they have icons the day they land. When the language registry (`rapidr-lang`) lands, the build reads its components instead.
- **The IDE's commands** (`[commands]`: `file.save` → `save`), the markers, file extensions, project-tree kinds, symbol kinds and toolbox groups (with their members).
- The build fails, and so do the crate's tests (`every_component_type_has_an_icon`, `every_command_and_kind_has_an_icon`, `toolbox_groups_hold_every_component_once`), when one of these has no icon.

## Pipeline

```sh
python3 design/icons/tools/build.py          # draw, optimize, check, generate
python3 design/icons/tools/build.py --check  # fail if anything is out of date (tools/regress.sh)
```

1. **Draw.** The geometry is Python, standard library only:
   - [`tools/kit.py`](tools/kit.py): the kit and the hinter;
   - [`tools/motifs.py`](tools/motifs.py): shared shapes;
   - `actions.py`, `glyphs.py`, `components.py`, `planned.py`, `files.py`, `symbols.py`, `groups.py`: one function per icon.

   The sources are written to `src/`. A source drawn in an editor instead is never overwritten if it holds `data-hand-drawn`.
2. **Optimize.** This is our own code ([`tools/build.py`](tools/build.py)). It strips the XML declaration, comments, `<title>`, `<desc>`, `<metadata>`, editor attributes and whitespace, and checks:
   - the root and the view box;
   - that every colour is a token;
   - that a monochrome icon uses only `currentColor`;
   - that there are no text, images, scripts or styles.
3. **Check the inventory** (see above).
4. **Generate** [`crates/rapidr-icons/src/generated.rs`](../../crates/rapidr-icons/src/generated.rs) (the palettes, the inventory, each drawing's place in the bundle) and `src/icons.deflate`: every optimized SVG, one after another, raw-deflated (459 KB become 42 KB). The crate inflates the bundle the first time an icon is drawn, so a program that never draws one carries only the compressed bytes.

Dev tools (they need `rsvg-convert` and Pillow):

- `tools/preview.py`: quick sheets while drawing;
- `tools/pixels.py`: magnified pixels, to check the hinting;
- `tools/sheets.py`: the review sheets, through the crate's own renderer;
- `tools/mockups.py`: the kernel mock-ups;
- `tools/export.py`: the manual's icons and the HTML catalog.

## Using them

- **Rust** ([`crates/rapidr-icons`](../../crates/rapidr-icons)):
  - `get`, `component`, `command`, `file`, `symbol` find an icon;
  - `themed_svg` gives an icon's SVG in a theme;
  - `render` gives its pixels for a logical size and a display scale.

  It depends on resvg alone and builds for wasm32.
- **The UI kernel**: `Painter::icon(name, rect, color, disabled)` ([`crates/rapidr-ui-kernel/src/icons.rs`](../../crates/rapidr-ui-kernel/src/icons.rs)) puts the icon into the display list as a picture made for the list's scale. It is identical on the desktop and the web.
- **RapidR programs** (additions; RapidQ has no such methods):

  ```basic
  DIM Glyph AS QBITMAP
  Glyph.LoadIcon "run", 16              ' Name$ [, Size [, Theme$]]
  RunButton.BMPHandle = Glyph

  DIM Pics AS QIMAGELIST
  Pics.Width = 16 : Pics.Height = 16
  Pics.AddIcon "QBUTTON"                ' Name$ [, Theme$]: at the list's size
  ```

  The bitmap keeps the icon as an SVG, drawn again at the screen's scale. An unknown name is an error the program sees, as a missing file is. These methods are rapidr-value's default feature `icons`; a build without it answers that it has no icons.
- **Docs, website, VS Code**: `tools/export.py` writes each icon's themed SVG and PNGs (`docs/manual/icons/`) and an HTML catalog.

Icons never replace names. A toolbar button keeps its hint, a toolbox item its type name, a tree node its text. Screen readers read those, never the picture.

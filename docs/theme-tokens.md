# RapidR's look: the theme tokens

`$THEME rapidr` is every program's look unless it names another (no `$THEME` at all is the same): RapidR Studio's own look, one set of tokens shared by Studio and by programs, in light, dark and high contrast, the same pixels on macOS, Windows, Linux and the web. `$THEME classic` is RapidQ's Windows look, exactly. This page says where each token comes from. The code is `crates/rapidr-value/src/theme.rs` (`RAPIDR`, `RAPIDR_DARK`, `RAPIDR_HIGH_CONTRAST`, `CLASSIC`) and `crates/rapidr-value/src/ide_theme.rs` (Studio's own chrome and the code editor).

## Names

| `$THEME` / `Application.Theme =` | Drawn in | `Application.Theme` reads |
|---|---|---|
| none, `rapidr`, `auto` | RapidR's look as the system is set: high contrast, dark, else light, and it follows the system when the user switches | `rapidr light`, `rapidr dark` or `rapidr high contrast` |
| `rapidr light`, `modern` (and `fluent`, `win11`, `macos` and the other older platform names) | RapidR light | `rapidr light` |
| `rapidr dark`, `dark` | RapidR dark | `rapidr dark` |
| `rapidr high contrast`, `highcontrast`, `hc` | RapidR high contrast | `rapidr high contrast` |
| `classic`, `rapidq`, `system`, `light`, `windows`, `win95`, `win98`, `win2k`, `base` | RapidQ's look | `classic` |
| a name no theme has | RapidR's look as the system is (the runtime says so once) | as above |

`RAPIDR_THEME` (the environment, or a web page's `rapidr_set_theme`) names the look of a program that names none; RapidR Studio's **Preview in Classic** (View menu, tool bar, command palette) uses it to run the program under development, and its designer, in the classic look without editing the source.

## The rules

- **Sizes never change between themes.** Component sizes and positions, client areas and a form's frame (a 1-pixel border, a 29-pixel title bar) are the same in every theme. An AutoSize label's size is RC.EXE's in every theme: its text is measured as RapidQ measures it (`theme::rapidq_metrics`), and where the theme's face is wider the text runs on past the label's edge rather than being cut. Tests: the kernel's `a_theme_changes_no_geometry`, and `node tests/theme_geometry.mjs` (every GUI program of `examples/`, `--corpus` RapidQ's own examples, in all four looks).
- **Colours.** RapidQ's system colours (`clBtnFace`, `clWindow`, `clHighlight` …, `&H80000000 + n`) are the theme's (`Theme::system_color`). An RGB value a program sets is painted exactly as written. Reading a colour back gives what the program set, as RC.EXE does.
- **Fonts.** RapidQ's default font (MS Sans Serif, Microsoft Sans Serif, MS Shell Dlg, no name) is Inter in RapidR's look and RapidR Sans (MS Sans Serif's metrics) in the classic one, at the same pixel size (8 pt: 11 px). Fonts a program names keep their face.
- **Contrast** (WCAG 2.x, unit tests `text_reads_on_every_theme` and `text_reads_on_every_ground`): text at least 4.5:1 in every theme, 7:1 in high contrast; disabled text, the focus ring, strong borders and the accent at least 3:1 against what they're on.

## The font size: the clipping audit

`cargo run --release -p rapidr-designer --example caption_audit` measures every literal caption in a component of a fixed size, over `examples/` and the RapidQ corpus (`~/Downloads/Rapidq/examples`, 529 programs), in RapidQ's font and in Inter at 11, 12 and 13 pixels (2026-10-08):

| Setting | Corpus: texts that clip (of 539) | New clips | `examples/` (of 83) |
|---|---|---|---|
| RapidQ's font (RapidR Sans 11 px) | 11 | — | 0 |
| **Inter 11 px** (chosen) | 36 | 26 | 0 |
| Inter 12 px | 67 | 56 | 1 |
| Inter 13 px | 89 | 78 | 1 |

Inter at MS Sans Serif's 11 pixels is about 10 % wider than RapidQ measured (median; 16 % at the 95th percentile). Every pixel more doubles the clipping, so RapidR's look draws the default font at 11 px; Inter's tall x-height makes it read as Windows 11's Segoe UI 9 pt.

## Where the colours come from

The brand palette (`design/brand/README.md`, the Pencil design `design/rapidr-brand.pen`, `theme::brand`): Ink `#0E1525`, Paper `#F6F8FC`, RapidR Blue `#2F5BFF`, Blue Deep `#1E3FD8`, Blue on Dark `#6E93FF`, Spark Cyan `#19C6E6`, Run Amber `#FFB224`, BASIC Teal `#12B48A` / Teal Deep `#0B7B5E`, Slate `#5B6478`, Mist `#C9D1E3`, Board `#E9EDF5`. The design file has no app mockups; the tokens are derived from the palette by the rules below. `mix(a, b, ‰)` is the per-channel blend `theme::mix`; `ink(‰)` is Ink lightened toward white (the dark variant's surface ramp).

### Surfaces, text and lines

| Token | What it colours | Light | Dark |
|---|---|---|---|
| `face` (clBtnFace) | forms, panels, the menu bar, tool bars | Paper | `ink(45)` |
| `window` (clWindow) | text boxes, lists, grids, trees | white | `ink(15)` (a step below the face: fields sit in it) |
| `control` / `_hot` / `_pressed` / `_disabled` | a button's fill | white / `#F3F6FB` / Board / Paper | `ink(95)` / `ink(135)` / `ink(65)` / `ink(60)` |
| `menu` | menus, drop-down lists, tooltips (popovers) | white | `ink(90)` |
| `text` (clWindowText, clBtnText) | text a program didn't colour | Ink | Board |
| `gray_text` (clGrayText) | what's disabled | `#858EA1` (3.1:1 on Paper) | `#6B7590` |
| `inactive_caption_text`, Studio's dim text | secondary text | Slate | `#8F99B0` |
| `border` | hairlines: fields, buttons, frames | `#D5DCE8` (Mist, lightened) | `ink(150)` |
| `border_hot` | the same under the mouse; the active window's frame | `#BCC4D6` | `ink(220)` |
| `border_strong` | rims that must show (3:1): a check box's | `#8A93A6` | `#7D879E` |
| `grid_lines`, `fixed_lines` | a grid's lines, its header's | `#E6EAF2`, `#D5DCE8` | `ink(110)`, `ink(180)` |
| `lines` | a tree's lines | Mist | `ink(250)` |

### What's chosen, the focus

| Token | What it colours | Light | Dark |
|---|---|---|---|
| `accent` | the default button, checked boxes, a toggle that's down, progress, a slider's value, a selected tab's underline | RapidR Blue | Blue on Dark (Ink text on it, 6.3:1) |
| `accent_hot`, `accent_pressed` | under the mouse, pressed | Blue → Blue Deep halfway, Blue Deep | Blue on Dark lightened 12 %, darkened toward Ink 15 % |
| `highlight` / `highlight_text` (clHighlight) | the selection in a focused list, tree, text | RapidR Blue / white (5.2:1) | RapidR Blue / white |
| `unfocused` | the selection when the list hasn't the focus | `#E3E8F1` | `ink(150)` |
| `selected` / `selected_text` | a soft selection: a grid's selected cells, the row under the mouse in a drop-down list, a menu's highlighted item | `mix(white, Blue, 120‰)` / Ink | `mix(ink(15), Blue, 300‰)` / white |
| `focus` | the focus: a 2-pixel ring inside a text box, a button, a combo box; a list, tree or grid gets a 1-pixel border in it, its selection in the accent saying the rest; a memo filling its window none | RapidR Blue (4.9:1 on Paper) | Blue on Dark |
| `hot_text` (clHotLight) | links, a hot-tracked tab | Blue Deep (7.6:1 on white) | Blue on Dark |

### Elevation and shape

| Token | Light | Dark |
|---|---|---|
| `radius` (controls) | 5 px | 5 px |
| `panel_radius()` (menus, drop-down lists, windows on the page) | 8 px | 8 px |
| `shadow_ink` / `shadow_alpha` (menus, drop-down lists, tooltips, web windows) | Ink at 38 / 255 | black at 140 / 255 |
| a raised control's rim | its border a step darker along the bottom | the same, toward black |

High contrast is Windows' High Contrast Black (white on black, cyan selection, yellow focus and hover, green disabled text), every control framed, 3-pixel focus rings, no shadows; it passes 7:1 for every text pair. Classic is RapidQ's Windows look, untouched.

### Studio's chrome (`ide_theme`)

The tool bar is the window's face with a `border` hairline under it; the status bar is quiet at rest (the face, dim text) and takes a state's colour while the program runs (BASIC Teal Deep), is paused (`#B45309`) or failed (`#C42B1C`), with white text. The start page is the `window` ground, its cards `control` with `border`. The light editor: white, Ink text, Blue Deep keywords, Teal Deep comments; the dark one: `ink(15)`, Board text, Blue on Dark keywords, Run Amber numbers. Every token is readable by name from a program: `Application.ThemeColor("statusbar.running")`.

## The window frame

A web page's windows and QFORMMDI's child windows are drawn by the kernel (`rapidr-ui-kernel/src/window_frame.rs`); a desktop form's window is the system's. In every look the frame is a 1-pixel border and a 29-pixel title bar.

- RapidR's look: the title bar is the window's surface, a hairline under it and around the window, corners `panel_radius()`, the title in Inter semibold, dimmed when the window isn't active; thin glyph buttons 40 pixels wide (the close button red under the mouse). On the web the window is lifted off the page by a soft CSS shadow, as the desktop's window system gives its own windows one.
- Classic: the title bar in the caption colour, the title in MS Sans Serif bold, Windows' own 16 × 14 bevelled buttons with their pixel glyphs, centred in the bar. At 2× they are the same shapes twice as big, never bigger buttons.

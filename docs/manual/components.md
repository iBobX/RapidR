# Components and objects

A RapidR program builds its windows from **components** — forms, buttons,
lists, grids, timers — and uses **objects** that have no window (fonts,
bitmaps, streams, string lists, databases). RapidR has every component and
object RapidQ has except OLE (QOLECONTAINER, QOLEOBJECT), and adds its own.

The full list, with both names and where each runs:
[reference/components.md](reference/components.md).

## RapidQ's names

RapidR's components are written `RButton`, `RForm`, `RStringGrid`, …. A
RapidQ program's `QBUTTON`, `QFORM`, `QSTRINGGRID`, … are the same
components (the RapidQ names), accepted everywhere. **Both names are one
component**: the same properties, methods, events, defaults, look and
behaviour, on every runtime. A few RapidQ names map to another R name:
`QGAUGE` is `RProgressBar`, `QOUTLINE` (Windows 3.1's tree) is `RTreeView`,
and RAPIDQ2.INC's `COMPORT` is `RComPort`.

- Mix them freely: an `RForm` can hold a `QBUTTON`, a `QFORM` an `RPlot`;
  `DIM`, `CREATE`, `EXTENDS`, parameters (`Sender AS RButton`) and arrays
  accept either name.
- **RapidR-only components** have only an R name: `RSQLite`, `RJson`,
  `RHttp`, `RNum`, `RDataFrame`, `RPlot`, `RCodeEditor`, the web components
  (`RWebView`, `RDOM`, …) and others.
- A program written with RapidQ's names, using only RapidQ's components and
  members, stays a plain RapidQ program, which RapidQ's own compiler still
  builds. To bring one over to RapidR's names, import it (RapidR Studio's
  File ▸ Import RapidQ Project or File…, or `rapidr import-rapidq`): a
  converted copy and a report, the original left alone —
  [Importing RapidQ programs](importing-rapidq.md).
- RapidR Studio shows RapidR's names (the toolbox, the inspector, Help,
  completion); what it adds to a file written with RapidQ's names is written
  with RapidQ's names, so a file never mixes them.
- Because a leading Q is read as R whenever the R component exists,
  `QPLOT` is accepted and means `RPlot`, though RapidQ has no QPLOT. Prefer
  the R name for RapidR's own components.

More on how the two families coexist (and how the planned IDE writes them):
[docs/q-and-r-components.md](../q-and-r-components.md).

## Making components

```basic
CREATE Form AS RForm                 ' a component and its children
    Caption = "Orders"
    Width = 400 : Height = 300
    Center
    CREATE Grid AS RStringGrid
        Align = alClient
        ColCount = 3
    END CREATE
END CREATE

DIM Font AS RFont                    ' an object, or a component made later
Font.Name = "Arial" : Font.Size = 12
DIM Extra AS RButton
Extra.Parent = Form                  ' put on the form at run time
DIM Labels(1 TO 3) AS RLabel         ' arrays of components
```

Inside `CREATE … END CREATE`, `Name = value` sets the component's property
and a bare method name (`Center`, `AddItems "a", "b"`) calls it.
Properties are read and written with `Component.Property`, methods called
with `Component.Method args` (a SUB) or `x = Component.Method(args)`.
RapidQ calls some methods without parentheses for their value (`WHILE
DB.FetchRow`, `IF Dlg.Execute THEN`); RapidR does the same.

## Events

An event property names a SUB:

```basic
CREATE Button1 AS RButton
    OnClick = ButtonClick
END CREATE

SUB ButtonClick (Sender AS RButton)
    ShowMessage "Clicked " + Sender.Caption
END SUB
```

- Event handlers get RapidQ's arguments, with the component last as
  `Sender`: `OnKeyDown (Key, Shift)`, `OnKeyPress (Key)`, `OnMouseDown
  (Button, X, Y, Shift)`, `OnMouseMove (X, Y, Shift)`, a grid's `OnDrawCell
  (Col, Row, State, Rect)`, … A handler may declare fewer parameters.
- Handlers can be bound at run time too: `Btn(i).OnClick = Clicked`.
- Events run when the program waits: during `ShowModal`, `DOEVENTS` or a
  dialog. The program ends when its main code does, as in RapidQ: forms
  still open close with it and their timers stop.
- A form's first `Show` (or `ShowModal`) fires `OnResize`, `OnShow`,
  `OnResize`, as RapidQ does — lay a form out in `OnResize` and it is right
  when it appears. `OnPaint` comes after, when the program next waits. A size
  the program sets on a shown form (`Width`, `Height`, `ClientWidth`) fires
  `OnResize` at once; a new position fires nothing.
- True / false properties read `1` or `0` (RapidQ's rule), RapidR Studio's
  panel components too (`Console.ShowTabs`, `Tree.ShowFiles`).

```basic
Form.ShowModal          ' shows the form and waits until it closes
' or
Form.Show               ' shows it and goes on; to keep it, wait:
DO: DOEVENTS: LOOP UNTIL Closed    ' (Closed set by the form's OnClose)
```

## The global objects

| Object | What it has |
|---|---|
| `Application` | `ExeName`, `Path`, `Title`, `Icon`, `Terminate`, `Minimize`, `Theme` (RapidR's) |
| `Screen` | `Width`, `Height`, `ClientWidth` / `ClientHeight` (the work area), `MouseX` / `MouseY`, `Monitors`, `Cursor`, `PixelsPerInch` (96), `Scale` (RapidR's) |
| `Clipboard` | `Text`, `SetAsText`, `GetAsText(n)`, `Clear`, `HasFormat`, … — the system's clipboard on the desktop |
| `Printer` | RapidQ's printer object (and `LPRINT`): each document becomes a PDF, sent to the printer with CUPS' `lp` on macOS and Linux, the browser's print dialog on the web; on Windows, or with no printer, it is saved as a PDF in the current folder. `RAPIDR_PRINT_TO=<file or folder>` saves it there instead |
| `Mouse` | `X`, `Y` |

A program run by the RapidR Runtime sees itself as if it were built:
`Application.ExeName` and `Application.Path` name its file, and
`COMMAND$(n)` / `CommandCount` see only its own arguments, as in RapidQ
(`COMMAND$(0)` is the program's file; a bare `COMMAND$`, RapidR's, is the
arguments joined with spaces) — however it runs: `rapidr run`, `rapidr
run-bc`, a double-clicked file, a built executable. On the web they are the
page's query string's parts (`?a&b%20c`: `a`, `b c`).

## RapidR's additions to every component

These are RapidR's, and additive: a program that doesn't use them behaves
exactly as under RapidQ. Using one makes the program RapidR-only (RapidQ's
compiler doesn't know it).

| Addition | |
|---|---|
| `Anchors = akLeft + akRight` (`akTop`, `akBottom`) | keeps the component's distances to its parent's edges as the parent is resized (Delphi's Anchors; RapidQ has `Align` only) |
| `MinWidth`, `MinHeight`, `MaxWidth`, `MaxHeight` (also `Constraints.MinWidth`, …) | bound every size the component takes, from the program, the layout or the user |
| `AccessibleName`, `AccessibleDescription` | what a screen reader says, when the caption isn't enough |
| `Form.Scale`, `Screen.Scale`, `OnScaleChanged` | how fine the screen is (2 on a Retina screen, 1.5 at 150 %) |
| `Application.Theme`, `$THEME` | the look (below) |
| `AutoComplete` on an edit | the browser's autofill hint, on the web |

## Themes

Programs are drawn in RapidR's own look — RapidR Studio's — unless they
ask for another. It follows the system's setting (light, dark or high
contrast) and is the same on macOS, Windows, Linux and the web.

| `$THEME` / `Application.Theme =` | Look | `Application.Theme` reads |
|---|---|---|
| none, `RapidR`, `Auto` (the default) | RapidR's look as the system is set, and it follows the system when the user switches | `rapidr light`, `rapidr dark` or `rapidr high contrast` |
| `RapidR Light` (also `Modern`) | RapidR's look, light | `rapidr light` |
| `RapidR Dark` (also `Dark`) | RapidR's look on the brand's Ink | `rapidr dark` |
| `RapidR High Contrast` (also `HighContrast`) | Windows' High Contrast Black: white on black, thick focus rings | `rapidr high contrast` |
| `Classic` (also `System`, `Light`, `Windows`, `Win95`, `Win2K`, …) | RapidQ's exact Windows look | `classic` |

`$THEME Classic` at the top of a program gives it RapidQ's look exactly;
`Application.Theme = "rapidr dark"` switches while it runs. The
`RAPIDR_THEME` environment variable gives a theme to programs that name
none (RapidR Studio's **Preview in Classic** uses it).

A theme changes only how things are drawn, never a size or a place: a
component's Left, Top, Width and Height, a form's client area and an
AutoSize label's size are RapidQ's in every theme. RapidQ's default font
(MS Sans Serif) is drawn in Inter in RapidR's look, at the same pixel size;
fonts a program names keep their face. RapidQ's system colours
(`clBtnFace`, `clWindow`, `clHighlight` …) are the theme's; a colour the
program sets as RGB (`Color`, `Font.Color`) is painted as written, and
reading a colour back gives what the program set, as in RapidQ. Text
contrast meets WCAG AA in every theme and AAA in high contrast. Where each
colour comes from: [theme tokens](../theme-tokens.md).

## High-DPI screens

A program's coordinates are logical pixels (1/96 inch, as RapidQ's, and as
Windows at 100 %) on every screen, so old layouts stay as they were; everything is drawn at
the screen's real resolution — text, lines, shapes, pictures and SVG images
(accepted wherever RapidQ takes a bitmap) — so it is sharp on Retina and
4K screens. `Screen.PixelsPerInch` reads 96; `Screen.Scale`
tells the real ratio.

## Accessibility

Every program is usable from the keyboard and by screen readers without
the author doing anything:

- Tab moves through components in `TabOrder` (those with `TabStop`); `&`
  in a caption makes an Alt shortcut; Enter presses a form's `Default`
  button and Escape its `Cancel` button; F10 / Alt opens the menu bar.
- Each window publishes an accessibility tree (roles, names, values,
  states, actions) to the system's screen reader: VoiceOver on macOS,
  Narrator and NVDA (UI Automation) on Windows, AT-SPI on Linux (through
  AccessKit); in a browser, an ARIA mirror of the same tree.
- A component's name is its caption, or its `AccessibleName`.

## How it is drawn

RapidR draws every window itself, with its own UI kernel (on winit, vello
and parley; the screen-reader trees through AccessKit), the same way on
macOS, Windows, Linux and in the browser — so a program looks and measures
the same everywhere. What users expect from their system stays the
system's: the macOS menu bar, the Open / Save dialogs, the clipboard, the
tray icon. Without a GPU (or with a software-only one, as in many virtual
machines) RapidR draws on the CPU.

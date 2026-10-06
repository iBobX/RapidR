# Components and objects

A RapidQ program builds its windows from **components** — forms, buttons,
lists, grids, timers — and uses **objects** that have no window (fonts,
bitmaps, streams, string lists, databases). RapidR has every component and
object RapidQ has except OLE (QOLECONTAINER, QOLEOBJECT), and adds its own.

The full list, with both names and where each runs:
[reference/components.md](reference/components.md).

## Q names and R names

RapidQ calls its components `QFORM`, `QBUTTON`, `QSTRINGGRID`, …; RapidR
calls the same components `RFORM`, `RBUTTON`, `RSTRINGGRID`, …. **Both names
are one component**: the same properties, methods, events, defaults, look
and behaviour, on every runtime. A few RapidQ names map to another R name:
`QGAUGE` is `RPROGRESSBAR`, `QOUTLINE` (Windows 3.1's tree) is `RTREEVIEW`,
RAPIDQ2.INC's `COMPORT` is `RCOMPORT`.

- Mix them freely: a `QFORM` can hold an `RPLOT`, an `RFORM` a `QBUTTON`;
  `DIM`, `CREATE`, `EXTENDS`, parameters (`Sender AS QBUTTON`) and arrays
  accept either name.
- **RapidR-only components** have only an R name: `RSQLITE`, `RJSON`,
  `RHTTP`, `RNUM`, `RDATAFRAME`, `RPLOT`, `RCODEEDITOR`, the web components
  (`RWEBVIEW`, `RDOM`, …) and others.
- A program that only uses RapidQ's components and members stays a plain
  RapidQ program, which RapidQ's own compiler still builds.
- Because a leading Q is read as R whenever the R component exists,
  `QPLOT` is accepted and means `RPLOT`, though RapidQ has no QPLOT. Prefer
  the R name for RapidR's own components.

More on how the two families coexist (and how the planned IDE writes them):
[docs/q-and-r-components.md](../q-and-r-components.md).

## Making components

```basic
CREATE Form AS QFORM                 ' a component and its children
    Caption = "Orders"
    Width = 400 : Height = 300
    Center
    CREATE Grid AS QSTRINGGRID
        Align = alClient
        ColCount = 3
    END CREATE
END CREATE

DIM Font AS QFONT                    ' an object, or a component made later
Font.Name = "Arial" : Font.Size = 12
DIM Extra AS QBUTTON
Extra.Parent = Form                  ' put on the form at run time
DIM Labels(1 TO 3) AS QLABEL         ' arrays of components
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
CREATE Button1 AS QBUTTON
    OnClick = ButtonClick
END CREATE

SUB ButtonClick (Sender AS QBUTTON)
    ShowMessage "Clicked " + Sender.Caption
END SUB
```

- Event handlers get RapidQ's arguments, with the component last as
  `Sender`: `OnKeyDown (Key, Shift)`, `OnKeyPress (Key)`, `OnMouseDown
  (Button, X, Y, Shift)`, `OnMouseMove (X, Y, Shift)`, a grid's `OnDrawCell
  (Col, Row, State, Rect)`, … A handler may declare fewer parameters.
- Handlers can be bound at run time too: `Btn(i).OnClick = Clicked`.
- Events run when the program waits: during `ShowModal`, `DOEVENTS`, a
  dialog, or once the main program has run to its end while a form is
  open.

```basic
Form.ShowModal          ' shows the form and waits until it closes
' or
Form.Show               ' shows it; the program waits at its end
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
`Application.ExeName` and `Application.Path` name its file and `COMMAND$`
holds its arguments.

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

Programs are drawn in Windows' classic look — RapidQ's — unless they ask
for another:

| `$THEME` / `Application.Theme` | Look |
|---|---|
| `Classic` (the default) | Windows' classic look; also `System`, `Windows`, `Win95`, `Win2K`, … |
| `Modern` | flat, after Windows 11: rounded controls, an accent colour, focus rings, thin scroll bars |
| `Dark` | the modern look, dark |
| `HighContrast` | Windows' High Contrast Black: white on black, thick focus rings |
| `Auto` | the system's: high contrast or dark when it is, else modern |

`$THEME Modern` at the top of a program picks one; `Application.Theme =
"dark"` switches while it runs and reads the theme in use. The
`RAPIDR_THEME` environment variable gives a theme to programs that name
none. A theme changes only how things are drawn — never a size, a place
or a font — and colours the program sets (`Color`, `Font.Color`) stay its
own. Older theme names from earlier RapidR versions (`Fluent`, `Aqua`,
`GTK`, …) still work and map to classic or modern.

## High-DPI screens

A program's coordinates are RapidQ's pixels (1/96 inch, as Windows at 100 %)
on every screen, so old layouts stay as they were; everything is drawn at
the screen's real resolution — text, lines, shapes, pictures and SVG images
(accepted wherever RapidQ takes a bitmap) — so it is sharp on Retina and
4K screens. `Screen.PixelsPerInch` reads RapidQ's 96; `Screen.Scale`
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

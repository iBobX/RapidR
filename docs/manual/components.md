# Components and objects

A RapidR program builds its windows from **components** — forms, buttons,
lists, grids, timers — and uses **objects** that have no window (fonts,
bitmaps, streams, string lists, databases). RapidR has all the classic components and
objects except OLE (QOLECONTAINER, QOLEOBJECT), and adds its own.

The full list, with both names and where each runs:
[reference/components.md](reference/components.md).

## The older Q names

RapidR's components are written `RButton`, `RForm`, `RStringGrid`, ….
Older programs use `QBUTTON`, `QFORM`, `QSTRINGGRID`, …: the
same components, accepted everywhere ([the full list](reference/classic-names.md)). **Both names are one
component**: the same properties, methods, events, defaults, look and
behaviour, on every runtime. A few Q names map to another R name:
`QGAUGE` is `RProgressBar`, `QOUTLINE` (Windows 3.1's tree) is `RTreeView`,
and `COMPORT` is `RComPort`.

- Mix them freely: an `RForm` can hold a `QBUTTON`, a `QFORM` an `RPlot`;
  `DIM`, `CREATE`, `EXTENDS`, parameters (`Sender AS RButton`) and arrays
  accept either name.
- **RapidR-only components** have only an R name: `RSQLite`, `RJson`,
  `RHttp`, `RNum`, `RDataFrame`, `RPlot`, the linked data components `RCSV`,
  `RDataSource` and `RDBGrid` ([Data science](data-science.md)), `RAI`
  ([The AI assistant and RAI](ai-assistant.md)), `RMarkdownView`
  ([Markdown](markdown.md)), `RCodeEditor`, the web components
  (`RWebView`, `RDOM`, …) and others.
- A program written with Q names, using only the classic components and
  members, stays a plain classic program, which the older compiler still
  builds. To bring one over to RapidR's names, import it (RapidR Studio's
  File ▸ Import RapidQ Project or File…, or `rapidr import-rapidq`): a
  converted copy and a report, the original left alone —
  [Importing older programs](importing-rapidq.md).
- RapidR Studio shows RapidR's names (the toolbox, the inspector, Help,
  completion); what it adds to a file written with Q names is written
  with Q names, so a file never mixes them.
- Because a leading Q is read as R whenever the R component exists,
  `QPLOT` is accepted and means `RPlot`, though there never was a QPLOT. Prefer
  the R name for RapidR's own components.

More on how the two families coexist (and how RapidR Studio writes them):
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
Some methods can be called without parentheses for their value (`WHILE
DB.FetchRow`, `IF Dlg.Execute THEN`).

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

- Event handlers get the classic arguments, with the component last as
  `Sender`: `OnKeyDown (Key, Shift)`, `OnKeyPress (Key)`, `OnMouseDown
  (Button, X, Y, Shift)`, `OnMouseMove (X, Y, Shift)`, a grid's `OnDrawCell
  (Col, Row, State, Rect)`, … A handler may declare fewer parameters.
- Handlers can be bound at run time too: `Btn(i).OnClick = Clicked`.
- Events run when the program waits: during `ShowModal`, `DOEVENTS` or a
  dialog. The program ends when its main code does: forms
  still open close with it and their timers stop.
- A form's first `Show` (or `ShowModal`) fires `OnResize`, `OnShow`,
  `OnResize` — lay a form out in `OnResize` and it is right
  when it appears. `OnPaint` comes after, when the program next waits. A size
  the program sets on a shown form (`Width`, `Height`, `ClientWidth`) fires
  `OnResize` at once; a new position fires nothing.
- Under `ShowModal`, a `Close` (or a `ModalResult`) in the form's own
  `OnShow` is lost: the form stays open, and gets its
  `OnClose` when it's closed later. Close it from a timer or a button
  instead.
- True / false properties read `1` or `0`, RapidR Studio's
  panel components too (`Console.ShowTabs`, `Tree.ShowFiles`).

```basic
Form.ShowModal          ' shows the form and waits until it closes
' or
Form.Show               ' shows it and goes on; to keep it, wait:
DO: DOEVENTS: LOOP UNTIL Closed    ' (Closed set by the form's OnClose)
```

### The focus: OnEnter and OnExit

Every component that takes the keyboard focus — an edit, a button, a check
box, a list, a grid … — has **OnEnter** (it got the focus) and **OnExit**
(it lost it to another component of its form). They come with Tab and
Shift+Tab, a click, `SetFocus` and a label's shortcut; the component that
has the focus when its form shows gets its OnEnter then. The one leaving
hears OnExit first, then the one arriving its OnEnter, both before what a
click does (OnMouseDown, OnClick). Switching to another window or app and
back fires neither, nor does hiding or disabling the focused component.

```basic
CREATE Age AS REdit
    OnExit = AgeExit
END CREATE

SUB AgeExit
    IF VAL(Age.Text) < 1 THEN Advice.Caption = "Type your age in years"
END SUB
```

[focus_events.rr](../../examples/gui/focus_events.rr) shows both. They are
RapidR's own: the classic components have OnEnter only on their lists
(a QFILELISTBOX's never fires; binding a QLISTBOX's stops the program) and no
OnExit, so no older program changes.

### Events your own code fires

Setting some properties from code fires the event the user's change fires,
inside the assignment — before the next line runs —, exactly as the classic
components do (the compatibility suite covers it):

| The program sets | Fires |
|---|---|
| `Edit.Text` or `Edit.SelText` (the text changes) | OnChange |
| `RichEdit.Text` (the text changes) | OnChange: once before its form has shown, **twice** after (the rich edit hears its own change and the text's) |
| `RichEdit.SelText` | OnChange |
| `CheckBox.Checked` (on or off, a change) | OnClick |
| `RadioButton.Checked = 1` (it was off) | OnClick — the button it turns off hears nothing |
| `ScrollBar.Position` (a change) | OnChange (not OnScroll) |
| the same value again; `RadioButton.Checked = 0` | nothing |
| `ComboBox.ItemIndex`, `ComboBox.Text`, `ListBox.ItemIndex`, `TrackBar.Position`, `TabControl.TabIndex` | nothing |

A handler that sets its own component's value runs itself again, and code
that fills a form fires the handlers it binds: keep a flag while you do,
as [csv_explorer.rr](../../examples/data/csv_explorer.rr) does with `Busy`.
Not yet the same as in the classic components: there, `RichEdit.AddStrings` fires OnChange, and a
QFILELISTBOX's OnChange comes when it first reads its folder.

### A run-time error in a handler

The handler stops at the line that failed — and so do the
handlers that fired it (a click whose `Edit.Text = …` ran the failing
OnChange) — and the program goes on: its windows keep answering, its timers
keep ticking, the next click runs the handler again. RapidR writes the
error where you see it (the program's error output, Studio's Output);
the classic components say nothing. In an event that ran inside a `DOEVENTS` the error
also stops the code that called `DOEVENTS`: a main program's
`DO: DOEVENTS: LOOP` ends there, the program with it. An
error in the main program ends the program. Under Studio's debugger the
program first stops at the line (see [Debugging](debugging.md#when-the-program-fails)).

### Windows messages: a form's WndProc

A form's `WndProc` hears the messages Windows sends or posts to the form's
window: `WndProc = MySub` in the form's CREATE (or
`Form.WndProc = MySub`), and `MySub (hWnd, uMsg, wParam, lParam)` runs for
each one. On Windows that is every message of the window: the system's
(`WM_SIZE`, `WM_MOVE`, `WM_ACTIVATE`, `WM_PAINT`, the mouse and the keys)
and the ones a program or a driver sends it. A program's own messages use
the numbers from `WM_USER` (&H400) on:

```basic
CONST WM_USER = &H400
DECLARE FUNCTION PostMessageA LIB "user32" ALIAS "PostMessageA" (hWnd AS LONG, uMsg AS LONG, wParam AS LONG, lParam AS LONG) AS LONG
DECLARE SUB MyWndProc (hWnd AS LONG, uMsg AS LONG, wParam AS LONG, lParam AS LONG)
DECLARE SUB PostClick

CREATE Form AS RForm
    WndProc = MyWndProc
    CREATE PostBtn AS RButton
        Caption = "Post": OnClick = PostClick
    END CREATE
END CREATE

SUB PostClick
    PostMessageA(Form.Handle, WM_USER + 1, 7, 9)
END SUB

SUB MyWndProc (hWnd AS LONG, uMsg AS LONG, wParam AS LONG, lParam AS LONG)
    IF uMsg = WM_USER + 1 THEN Form.Caption = "Got it: " + STR$(wParam) + ", " + STR$(lParam)
END SUB

Form.ShowModal
```

`OnMessage` is RapidR's other name for `WndProc`: `OnMessage = MyWndProc` in
the form's CREATE (or `Form.OnMessage = MyWndProc`) is the same event, with
the same four parameters, heard in the same places, and the same on every
system. Use whichever name reads better to you. A program that binds both
on one form hears its messages in the handler bound last.

A MIDI keyboard works the same way: winmm's `midiInOpen` with
`CALLBACK_WINDOW` (&H10000) and the form's `Handle` sends each MIDI
message to the form as `MM_MIM_DATA` (&H3C3), the MIDI bytes in lParam
(status in the low byte, note next, velocity above it). Open the device
once the form is showing (from a button, or the first message WndProc
gets): before its window exists, `Form.Handle` is RapidR's own number.

What to know:

- `hWnd` is the form's `Handle`; `uMsg`, `wParam` and `lParam` are 32-bit
  numbers (`&H80000000` arrives as -2147483648).
- The handler runs just after Windows has handled the message, in the
  order the messages came; what it returns doesn't matter. `DOEVENTS`
  inside it lets the next messages in.
- Messages come from the moment the form's window shows. The classic
  WndProc is called already while CREATE builds the form, so a handler that
  uses a control created later in the block stops such a program; RapidR's
  doesn't.
- Every form has its own WndProc. (The classic one is for the whole program:
  the last one bound hears every form's messages.)
- A message whose lParam points to data (`WM_COPYDATA`,
  `WM_GETMINMAXINFO`) can't be read: `PEEK` reaches only the program's own
  memory.
- RapidR draws a form's controls in the form's own window, so the mouse
  over a control is the form's `WM_MOUSEMOVE` too (the classic controls are
  windows of their own). RapidR's redraws of its controls are not handed
  over as `WM_PAINT`.
- On macOS, Linux and the web there are no Windows messages: `WndProc`
  hears only the form's [system tray icon](directx-and-media.md#the-system-tray).
  A control's `WndProc` is never called, on any system.

The examples in [examples/windows/](../../examples/windows/) list a form's
messages as they come, post `WM_USER + 1`, read a MIDI keyboard, and send a
MIDI SysEx message (`midi_sysex_out.rr`: it also shows how a TYPE is laid
out for a 64-bit Windows structure, `MIDIHDR`; see
[Windows DLL calls](../windows-dll-calls.md)).

## Message boxes: the safe answer on Enter

Enter presses a message box's first button. For a question
whose first answer can't be undone, name the safe one with `MESSAGEDLG`'s
fifth argument, the answer of the
button Enter presses:

```basic
IF MESSAGEDLG("This deletes 1,204 rows.", mtWarning, mbYes OR mbNo, 0, mrNo) = mrYes THEN
    DeleteRows
END IF
```

- That button has the focus as the box opens, and Tab still goes left to
  right from it. The same on the desktop and on the web, in every theme.
- Escape and the close box answer as they always did: Cancel if there is
  one, else No.
- Its `mb` flag works too (`mbNo`), when no button answers that number.
  Without the argument, or naming a button the box hasn't, the first
  button is the default.
- `MESSAGEBOX` takes Windows' own flags for it:
  `MB_DEFBUTTON2` (`&H100`) or `MB_DEFBUTTON3` (`&H200`), added to the
  buttons: `MESSAGEBOX("Delete it?", "Notes", MB_YESNO OR &H100)`.

![A warning whose No button is the default: No has the focus ring and the accent colour, Yes is plain](images/components/message-default-no.png)

The example is [transactions.rr](../../examples/data/transactions.rr)'s close question.

In a test, `RAPIDR_TEST_MESSAGE_DIALOG=__enter` answers a box with its
default button, as Enter would.

## Lists scroll to what you pick

A list box shows the item your program picks, as Windows' list boxes do:
setting `ItemIndex` scrolls the least that brings the item into view (nothing
moves when it's already there), and setting `TopIndex` makes that item the
first row shown — as far as the list can scroll: `TopIndex` of the last item
shows the list's last page. Reading `TopIndex` gives the first row shown, also
after the user scrolled. It works the same in a list in `Columns` (the list
scrolls across), in an owner-drawn list, and on the desktop, in a native
build and on the web.

```basic
' a message list: each new message added, then picked and shown
Messages.AddItems "Hello, " + Name
Messages.ItemIndex = Messages.ItemCount - 1
```

(`TopIndex` reads the list as it was last drawn: set it, and it reads what
the list shows once the window has drawn it, a moment later.)

## The global objects

| Object | What it has |
|---|---|
| `Application` | `ExeName`, `Path`, `Title`, `Icon`, `Terminate`, `Minimize`, `Theme` (RapidR's) |
| `Screen` | `Width`, `Height`, `ClientWidth` / `ClientHeight` (the work area), `MouseX` / `MouseY`, `Monitors`, `Cursor`, `PixelsPerInch` (96), `Scale` (RapidR's) |
| `Clipboard` | `Text`, `SetAsText`, `GetAsText(n)`, `Clear`, `HasFormat`, … — the system's clipboard on the desktop |
| `Printer` | The printer object (and `LPRINT`): each document becomes a PDF, sent to the printer with CUPS' `lp` on macOS and Linux, the browser's print dialog on the web; on Windows, or with no printer, it is saved as a PDF in the current folder. `RAPIDR_PRINT_TO=<file or folder>` saves it there instead |
| `Mouse` | `X`, `Y` |

A program run by the RapidR Runtime sees itself as if it were built:
`Application.ExeName` and `Application.Path` name its file, and
`COMMAND$(n)` / `CommandCount` see only its own arguments
(`COMMAND$(0)` is the program's file; a bare `COMMAND$`, RapidR's, is the
arguments joined with spaces) — however it runs: `rapidr run`, `rapidr
run-bc`, a double-clicked file, a built executable. On the web they are the
page's query string's parts (`?a&b%20c`: `a`, `b c`).

## RapidR's additions to every component

These are RapidR's, and additive: a program that doesn't use them behaves
exactly as before. Using one makes the program RapidR-only (the older
compiler doesn't know it).

| Addition | |
|---|---|
| `Anchors = akLeft + akRight` (`akTop`, `akBottom`) | keeps the component's distances to its parent's edges as the parent is resized (the classic components have `Align` only) |
| `MinWidth`, `MinHeight`, `MaxWidth`, `MaxHeight` (also `Constraints.MinWidth`, …) | bound every size the component takes, from the program, the layout or the user |
| `AccessibleName`, `AccessibleDescription` | what a screen reader says, when the caption isn't enough |
| `Form.Scale`, `Screen.Scale`, `OnScaleChanged` | how fine the screen is (2 on a Retina screen, 1.5 at 150 %) |
| `Application.Theme`, `$THEME`, `Application.OnThemeChange` | the look, and when it changes (below) |
| `TabStop` on a panel | the panel takes the keyboard focus: a clickable card ([Accessibility](#accessibility)) |
| `TabStop` on a label | the label is a link: Tab reaches it, Enter or Space clicks it ([Accessibility](#accessibility)) |
| `Application.Announce Text [, Assertive]` | a screen reader says Text without the focus moving ([below](#telling-screen-readers-about-news-applicationannounce)) |
| `AutoComplete` on an edit | the browser's autofill hint, on the web |
| `Placeholder = "Search the help"` on an edit | the grey line shown while the box is empty, saying what to type; never read back as the text, gone when the user types (RapidR Studio's Help box uses it) |
| `Form.OnDropFiles (Files AS STRING)` | files dragged from the computer and dropped on the window (below) |

### Files dropped on a form

`OnDropFiles` (RapidR's; the classic components have no such event) hears files dragged from
Finder, File Explorer or a Linux file manager — in a browser, from the
computer — and dropped anywhere on the form's window. `Files` holds their
paths, one a line (`CHR$(10)` between them), in the order they came; a drop
of several files is one event. Open each as any file. In a browser a
dropped file is read into the program's own files under its name, so
`Files` holds names (`sales.csv`) the program opens the same way. A form
without the handler ignores drops.

```basic
CREATE Form AS RFORM
    Caption = "Drop files here"
    OnDropFiles = Dropped
    CREATE Info AS RLABEL
        Left = 8: Top = 8: Width = 300
    END CREATE
END CREATE

SUB Dropped (Files AS STRING)
    DIM i AS INTEGER
    FOR i = 1 TO TALLY(Files, CHR$(10)) + 1
        Info.Caption = Info.Caption + FIELD$(Files, CHR$(10), i) + "  "
    NEXT
END SUB

Form.ShowModal
```

The [CSV Explorer](data-science.md#tutorial-the-csv-explorer) example opens
the CSV file dropped on it.

## The mouse pointer

Every component shows the pointer its user expects, without a line of code:
the I-beam over a text box, memo, rich edit and a combo box's text; the
column-resize pointer (a double arrow with a bar) over a `RSPLITTER` between
two columns and over a grid's, list view's or header's column edge (row-resize
for a splitter between two rows and for a grid's row edge with `goRowSizing`); the diagonal and straight sizing arrows over
the edges and corners of a `RFORMMDI` child window; the sizing corner over a
status bar's grip. Studio's panels follow the same rule: the line between the
name and value columns of the property inspector, the splitters between
panels and between split editors, the handles of the form designer, links in
the Output panel, and a tab, file or toolbox item being carried (a closed
hand). In RapidR's look the divider you can drag also lights up under the
mouse. Set the component's `Cursor` (`crHandPoint`, `crSizeWE`, …) and your
choice wins; `crDefault` (0) means "the component's own".

**A splitter you can see.** In RapidR's look (light, dark and high
contrast) an `RSplitter` shows at rest without shouting: a quiet gap with a
soft 1-pixel line along its middle (crisp in high contrast) and a short
grip on that line, so people see there is something to drag. The grip has
at least 3:1 contrast against the gap. When the mouse rests on it for a
moment (passing over it doesn't flicker) and while it is dragged, the whole
splitter takes the accent colour. Studio's own splitters between its panes
look the same, without the line: the panes' own edges already show where
they meet (each pane of an `RDockManager` has a thin edge down both sides and
along the bottom, and what you put in a pane draws no border of its own
there). Your program's splitters keep the place and size you gave them;
only the drawing is new. The classic look draws the raised bar, as
before.
On the desktop this is the system's cursor, on the web the page's cursor; they
agree.

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
| `Classic` (also `System`, `Light`, `Windows`, `Win95`, `Win2K`, …) | The classic Windows look, exactly | `classic` |

`$THEME Classic` at the top of a program gives it the classic look exactly;
`Application.Theme = "rapidr dark"` switches while it runs. The
`RAPIDR_THEME` environment variable gives a theme to programs that name
none (RapidR Studio's **Preview in Classic** uses it).

### When the look changes

A program in RapidR's look (the default) changes look while it runs: when
the person using it switches the system to light, dark or high contrast —
macOS' Appearance and Increase Contrast, Windows'
app mode and High Contrast, the browser's colour scheme on the web — and
when the program sets `Application.Theme`. It opens in the system's look
from its first line. What RapidR draws follows by itself: every
component, and the system colours (`clBtnFace`, `clWindowText` …).

A colour the program *read* from the theme with `Application.ThemeColor`
is a number, though: it stays what it was. RapidR's
`Application.OnThemeChange` fires once the look has changed — take those
colours again there:

```basic
SUB TakeColors
    Card.Color = Application.ThemeColor("card")
    CardTitle.Font.Color = Application.ThemeColor("page.text")
    Now.Caption = "Theme: " + Application.Theme
END SUB

TakeColors                              ' the look it opens in
Application.OnThemeChange = TakeColors  ' and each look after it
```

It fires after the change, once (two changes in a moment fire it once,
for the look they end in), from the moment it's set: not for the look the
program opens in, so call your SUB once yourself, as above. A program
that named a look (`$THEME Dark`, `Application.Theme = "classic"`) keeps
it when the system switches; `Application.Theme = "rapidr"` follows the
system again. [themes.rr](../../examples/gui/themes.rr) shows it: switch
your system to dark while it runs, or pick a look with its buttons.

![themes.rr on a light system](images/themes/themes-light.png)
![The same window after the system switched to dark while it ran: the controls, the card taken again in OnThemeChange, "Theme: rapidr dark"; the yellow label's own colours stay](images/themes/themes-system-dark.png)

RapidR Studio does the same: it opens in your system's look and follows
it (View ▸ Theme ▸ RapidR, the default); View ▸ Theme ▸ Light, Dark, High
Contrast or Classic picks one instead. Every pane, the Welcome page, the
designer, the debugger and any dialog that's open take the new look at
once.

![RapidR Studio opened on a light system, after the system switched to dark: the Welcome page, its cards and links in the dark look](images/themes/studio-system-dark.png)

A theme changes only how things are drawn, never a size or a place: a
component's Left, Top, Width and Height, a form's client area and an
AutoSize label's size are the classic ones in every theme. The default font
(MS Sans Serif) is drawn in Inter in RapidR's look, at the same pixel size;
fonts a program names keep their face. The system colours
(`clBtnFace`, `clWindow`, `clHighlight` …) are the theme's; a colour the
program sets as RGB (`Color`, `Font.Color`) is painted as written, and
reading a colour back gives what the program set. A QFONT
you never gave a colour keeps the theme's text colour (its `Color` reads
`clWindowText`), so `Form.Font = MyFont` changes the face
and size of the form's text but never turns it black on a dark theme:

```basic
DIM UIFont AS RFont
UIFont.Name = "Inter": UIFont.Size = 10
Form.Font = UIFont                 ' the theme's text colour, light or dark
Notes.Font.Color = RGB(200, 0, 0)  ' a colour you set: painted as written
```

Text contrast meets WCAG AA in every theme (secondary text — hints,
placeholders — included) and AAA in high contrast; RapidR's tests check
it on what is drawn. Where each colour comes from: [theme
tokens](../theme-tokens.md).

## High-DPI screens

A program's coordinates are logical pixels (1/96 inch, as Windows at 100 %) on every screen, so old layouts stay as they were; everything is drawn at
the screen's real resolution — text, lines, shapes, pictures and SVG images
(accepted wherever a bitmap is) — so it is sharp on Retina and
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
- Menus are read too. A menu bar drawn in the window (Windows, Linux, the
  web; macOS uses the system's own) is a menu bar of menu items; a menu
  that is open — a drop-down, a submenu, a pop-up menu — is a menu with its
  items inside it: a check mark or a radio dot is "checked", a greyed item
  "disabled", its `ShortCut` is said after its name, an item with a submenu
  says it is collapsed or expanded, and the item the arrow keys chose has
  the screen reader's focus, so it is read as the keys move. A screen
  reader can also press an item, open or close a submenu and move to an
  item, as the keyboard and the mouse do. An item's `Hint` is its
  description.
- (RapidR's) A panel with `TabStop = 1` takes the focus like a button:
  Tab reaches it, the theme's focus ring shows around it, Enter or Space
  clicks it (`OnClick`), and a screen reader announces a button named by
  its `AccessibleName`. That's a clickable card — RapidR Studio's Welcome
  page is made of them. In an `RScrollBox`, Tab onto a component below its
  edge scrolls it into view. A panel without `TabStop` never takes the
  focus.
- (RapidR's) A label with `TabStop = 1` is a link: Tab reaches it, the
  theme's focus ring shows around its text, Enter or Space clicks it
  (`OnClick`), and a screen reader announces a link named by its
  `Caption` (or `AccessibleName`). RapidR Studio's Recent list is made of
  them. The link's text sits 4 pixels inside the label's rectangle, so the
  ring has room round the letters. A label without `TabStop` never takes the
  focus.
  `examples/gui/links.rr` shows it.

### Telling screen readers about news: Application.Announce

Some news has no focus to go with it: a search found 3 rows, a new card
appeared in a feed, a download finished, the connection was lost.
`Application.Announce` (RapidR's) has the screen reader say it, without
moving the focus from where the person is working:

```basic
Application.Announce "3 rows found"                    ' politely
Application.Announce "The connection was lost", True   ' at once
```

- Politely (the default): the screen reader says it when it has finished
  what it is saying. Use it for nearly everything.
- `Assertive` True: it interrupts and says it at once. Keep it for what
  can't wait — an error, a lost connection.
- The same text announced twice is said twice. Keep it short: a few words.
- On the desktop it goes through the focused window's live regions
  (AccessKit), to the system's screen reader; in a browser through the
  page's `aria-live` region. With no screen reader on, nothing happens and
  nothing shows.

The local API example
([examples/network/local_api.rr](../../examples/network/local_api.rr))
announces each request it answers.

## How it is drawn

RapidR draws every window itself, with its own UI kernel (on winit, vello
and parley; the screen-reader trees through AccessKit), the same way on
macOS, Windows, Linux and in the browser — so a program looks and measures
the same everywhere. What users expect from their system stays the
system's: the macOS menu bar, the Open / Save dialogs, the clipboard, the
tray icon. Without a GPU (or with a software-only one, as in many virtual
machines) RapidR draws on the CPU.

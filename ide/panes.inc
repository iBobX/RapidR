' panes.inc: the dock manager's panes, and the start page.
'
' Plug-in points (docs/ide-plan.md I1): each pane is registered by name with
' Dock.AddPane in SetUpDock — a component replaces a pane by taking its
' place there (same name, same Where):
'   ProjectTree   "Project"     left              RPROJECTTREE   (panels.inc)
'   Toolbox       "Toolbox"     below Project     RTOOLBOX       (panels.inc)
'   Inspector     "Properties"  right             RPROPERTYINSPECTOR (panels.inc)
'   OutlineTree   "Outline"     below Properties  (RLanguageService's outline)
'   OutputBox     "Output"      bottom            ROUTPUTCONSOLE: output, build,
'                                                 problems (panels.inc)
'   ImmediateBox  "Immediate"   with Output       RImmediateWindow (I6)
'   VarsTree, WatchPanel, StackTree, BpPanel      beside Output: Variables,
'                 Watch, Call Stack, Breakpoints (debug.inc SetUpDebug)
'   CodeDoc(i)    a source file's tab             RCodeEditor    (L-EDVIEW)
'   DesignDoc(i)  its form: the tab's Design view RDESIGNSURFACE (workspace.inc)
'   SearchPane    "Search"      with Project      Find in Files  (search.inc)
'   HelpPane      "Help"        with Properties   F1             (search.inc)

SUB SetUpDock
  Dock.AddPane(ProjectTree, "Project", "left", "explorer")
  Dock.AddPane(Toolbox, "Toolbox", "bottom:projecttree", "toolbox")
  Dock.AddPane(Inspector, "Properties", "right", "properties")
  Dock.AddPane(OutlineTree, "Outline", "bottom:inspector", "outline")
  Dock.AddPane(OutputBox, "Output", "bottom:documents", "output")
  Dock.AddPane(ImmediateBox, "Immediate", "tab:outputbox", "immediate")
  Dock.AddPane(Welcome, "Welcome", "documents", "list")
  Dock.ShowPane("outputbox")
END SUB

' ---- the start page ----------------------------------------------------------------
' Start: what to do first; Recent: the projects opened last (settings);
' Examples: RapidR's own, as cards in a grid that follows the page's width
' (each also in the command palette, for the keyboard).

DIM StartIcon(4) AS RImage
DIM StartBmp(4) AS RBitmap
DIM StartLink(4) AS RLabel
' (each Start row a panel: Tab reaches it, Enter or Space runs it, a
' screen reader names it — as the example cards)
DIM StartRow(4) AS RPanel
DIM StartIconName(4) AS STRING
DIM RecentLink(6) AS RLabel
' (S2) the start-up's offer to reopen the last project: a card at the top of
' Start, focused (Enter reopens it); Start and Recent below it move down by
' StartShift while it shows
DIM ReopenRow AS RPanel
DIM ReopenIcon AS RImage
DIM ReopenBmp AS RBitmap
DIM ReopenTitle AS RLabel
DIM ReopenText AS RLabel
DIM StartShift AS INTEGER
DIM ExCard(16) AS RPanel
DIM ExIcon(16) AS RImage
DIM ExBmp(16) AS RBitmap
DIM ExTitle(16) AS RLabel
DIM ExText(16) AS RLabel
DIM ExampleFile(16) AS STRING
DIM ExampleWhat(16) AS STRING
DIM ExampleCount AS INTEGER
' More examples than the page has room for: only in "Open an Example" (the
' command palette), not cards on this page
DIM MoreFile(40) AS STRING
DIM MoreWhat(40) AS STRING
DIM MoreCount AS INTEGER
DIM LogoBmp AS RBitmap

SUB AddStart(k AS INTEGER, Icon AS STRING, Caption AS STRING)
  StartRow(k).Parent = WelcomePage
  StartRow(k).Left = 44: StartRow(k).Top = 164 + k * 28: StartRow(k).Width = 234: StartRow(k).Height = 24
  StartRow(k).BevelOuter = 0
  StartRow(k).Tag = k + 1
  StartRow(k).Cursor = -21
  StartRow(k).AccessibleName = Caption
  StartRow(k).OnClick = StartRowPicked
  StartRow(k).TabStop = 1
  StartIcon(k).Parent = StartRow(k)
  StartIcon(k).Left = 4: StartIcon(k).Top = 4: StartIcon(k).Width = 16: StartIcon(k).Height = 16
  StartIconName(k) = Icon
  StartLink(k).Parent = StartRow(k)
  StartLink(k).Left = 30: StartLink(k).Top = 2: StartLink(k).Width = 200: StartLink(k).Height = 20
  StartLink(k).Caption = Caption
  StartLink(k).Cursor = -21
  StartLink(k).Tag = k + 1
  StartLink(k).OnClick = WelcomeLink
END SUB

SUB AddExample(File AS STRING, What AS STRING)
  DIM i AS INTEGER
  i = ExampleCount
  IF i > 16 THEN EXIT SUB
  ExampleFile(i) = File
  ExampleWhat(i) = What
  ExCard(i).Parent = WelcomePage
  ExCard(i).BevelOuter = 0
  ExCard(i).Tag = i
  ExCard(i).Cursor = -21
  ExCard(i).AccessibleName = "Example " + BaseName(File) + ": " + What
  ExCard(i).OnClick = CardPicked
  ' (Tab reaches each card, Enter or Space opens it)
  ExCard(i).TabStop = 1
  ExIcon(i).Parent = ExCard(i)
  ExIcon(i).Left = 14: ExIcon(i).Top = 14: ExIcon(i).Width = 24: ExIcon(i).Height = 24
  ExTitle(i).Parent = ExCard(i)
  ExTitle(i).Left = 48: ExTitle(i).Top = 12: ExTitle(i).Width = 168: ExTitle(i).Height = 18
  ExTitle(i).Caption = FileTitle(File)
  ExTitle(i).Tag = i
  ExTitle(i).OnClick = CardLabelPicked
  ExText(i).Parent = ExCard(i)
  ExText(i).Left = 48: ExText(i).Top = 32: ExText(i).Width = 168: ExText(i).Height = 18
  ExText(i).Caption = What
  ExText(i).Tag = i
  ExText(i).OnClick = CardLabelPicked
  ExampleCount = ExampleCount + 1
END SUB

SUB AddMoreExample(File AS STRING, What AS STRING)
  IF MoreCount > 40 THEN EXIT SUB
  MoreFile(MoreCount) = File
  MoreWhat(MoreCount) = What
  MoreCount = MoreCount + 1
END SUB

' The examples that come with RapidR (examples/, as `rapidr examples` lists them).
SUB FillExamples
  ' (S2: the offer to reopen shows only when ShowReopen makes it)
  ReopenRow.Visible = 0
  AddStart 0, "file.newProject", "New Project..."
  AddStart 1, "file.open", "Open Project or File..."
  AddStart 2, "file.import", "Import a RapidQ Program..."
  AddStart 3, "RFORM", "Open an Example"
  AddStart 4, "view.commandPalette", "All Commands"
  ExampleCount = 0
  AddExample "data/csv_explorer.rr", "Drop a CSV, see it charted"
  AddExample "data/linked_data.rr", "CSV, grid and edits, linked"
  AddExample "data/million_rows.rr", "A million rows, analysed"
  AddExample "data/json_source.rr", "JSON records in a grid"
  AddExample "gui/hello_form.rr", "A form, a button, its OnClick"
  AddExample "gui/focus_events.rr", "The focus: OnEnter, OnExit"
  AddExample "gui/menus.rr", "Main and pop-up menus"
  AddExample "gui/dialogs.rr", "Open, save, colour, font"
  AddExample "gui/richtext.rr", "A rich-text editor (.rtf)"
  AddExample "gui/pantry.rr", "A list and a grid together"
  AddExample "gui/themes.rr", "The themes, switched live"
  AddExample "gui/sales_dashboard.rr", "A big window: a dashboard"
  AddExample "data/dataframe.rr", "A data frame and a chart"
  AddExample "debugging/shop.rr", "Debug it: values as trees"
  AddExample "ai/rai_models.rr", "Ask an AI: models, an answer"
  AddExample "rapidq/notepad.bas", "A program in the classic style"
  AddExample "basics/hello.rr", "PRINT, FOR, SUB, FUNCTION"
  ' (the showcase: in Open an Example, not on the page)
  MoreCount = 0
  AddMoreExample "studio/helpers.rr", "The code shows its own values: colours, pictures, fonts"
  AddMoreExample "showcase/fireworks.rr", "Click the sky: rockets and thousands of sparks"
  AddMoreExample "showcase/notes.rr", "A Markdown notes app, live preview"
  AddMoreExample "showcase/snake.rr", "A snake game, for the keyboard alone"
  AddMoreExample "showcase/same_numbers.rr", "Four pictures, the very same numbers"
  AddMoreExample "showcase/monte_carlo_pi.rr", "Pi by throwing darts, live"
  AddMoreExample "showcase/spin3d.rr", "A lit 3D planet with a ring and a moon"
  AddMoreExample "showcase/signal_lab.rr", "A signal generator, an oscilloscope and an FFT spectrum"
  AddMoreExample "showcase/midi_visualizer.rr", "A 3D scene that dances to a song (reads the MIDI file itself)"
  AddMoreExample "showcase/make_song.rr", "Composes the visualizer's song and writes a .mid file"
  AddMoreExample "gui/value_tree.rr", "Values as the debugger shows them: icons, swatches, changes"
  AddMoreExample "data/million_rows_async.rr", "A million rows from SQLite in the background, an SQL editor"
  AddMoreExample "data/transactions.rr", "A transaction you can see; closing asks, Enter answers No"
  AddMoreExample "network/local_api.rr", "A web API in your program (RHttpServer), this computer only"
  AddMoreExample "gui/markdown_code.rr", "Code in Markdown, coloured: SQL and BASIC"
  AddMoreExample "gui/links.rr", "Labels as links: Tab, a focus ring, Enter or Space"
  AddMoreExample "studio/log_copy.rr", "A log to select and copy from: Ctrl+C, Copy Line, Copy All"
  ' (the recipes: short programs to start from — the assistant starts from them too)
  AddMoreExample "recipes/calculator.rr", "Recipe: a calculator with sqrt, sin, log, one handler for every key"
  AddMoreExample "recipes/tip_calculator.rr", "Recipe: a tip calculator"
  AddMoreExample "recipes/stopwatch.rr", "Recipe: a stopwatch (RTimer)"
  AddMoreExample "recipes/todo_list.rr", "Recipe: a to-do list, add and remove"
  AddMoreExample "recipes/unit_converter.rr", "Recipe: a unit converter"
  AddMoreExample "recipes/quiz_game.rr", "Recipe: a quiz with a score"
  AddMoreExample "recipes/drawing_pad.rr", "Recipe: a drawing pad (RCanvas)"
  AddMoreExample "recipes/password_generator.rr", "Recipe: a password generator"
  AddMoreExample "recipes/csv_viewer.rr", "Recipe: a CSV file in a grid"
  AddMoreExample "recipes/server_manager.rr", "Recipe: a list with Start, Stop and Restart"
  LayOutWelcome
END SUB

' (the brand and the icons in the current theme, again after a switch)
SUB LoadWelcomeIcons
  DIM k AS INTEGER
  IF INSTR(Application.Theme, "dark") > 0 OR INSTR(Application.Theme, "contrast") > 0 THEN
    LogoBmp.LoadFromFile HomeDir + "ide/assets/rapidr-lockup-dark.svg"
  ELSE
    LogoBmp.LoadFromFile HomeDir + "ide/assets/rapidr-lockup.svg"
  END IF
  WelcomeLogo.BMPHandle = LogoBmp
  FOR k = 0 TO 4
    StartBmp(k).LoadIcon StartIconName(k), 16
    StartIcon(k).BMPHandle = StartBmp(k)
  NEXT
  FOR k = 0 TO ExampleCount - 1
    IF FileExt(ExampleFile(k)) = "bas" THEN ExBmp(k).LoadIcon "files/bas", 24 ELSE ExBmp(k).LoadIcon "files/rr", 24
    ExIcon(k).BMPHandle = ExBmp(k)
  NEXT
END SUB

' The cards in as many columns as the page has room for (200 pixels or
' more each, sharing the width), the Examples heading over them. The page
' (WelcomePage, in the Welcome scroll box) as tall as what's on it: when
' that's more than the window shows, the box's scroll bar takes 17 pixels
' of the width and the page scrolls.
SUB LayOutWelcome
  DIM cols AS INTEGER, k AS INTEGER, room AS INTEGER, w AS INTEGER, x0 AS INTEGER
  DIM rows AS INTEGER, pitch AS INTEGER, tall AS INTEGER
  DIM pageW AS INTEGER, pageH AS INTEGER, bottom AS INTEGER, pass AS INTEGER
  x0 = 290
  pageW = Welcome.Width
  FOR pass = 1 TO 2
    room = pageW - x0 - 32
    cols = (room + 12) \ 212
    IF cols < 1 THEN cols = 1
    IF cols > 4 THEN cols = 4
    w = (room - (cols - 1) * 12) \ cols
    IF w > 300 THEN w = 300
    ' (a short page: the cards closer together, down to 56 apart; still
    ' too short, it scrolls)
    rows = (ExampleCount + cols - 1) \ cols
    pitch = 70: tall = 58
    IF rows > 0 THEN
      IF 168 + rows * pitch - 12 > Welcome.Height - 8 THEN
        pitch = (Welcome.Height - 170) \ rows
        IF pitch < 56 THEN pitch = 56
        IF pitch > 70 THEN pitch = 70
        tall = pitch - 6
        IF tall > 58 THEN tall = 58
      END IF
    END IF
    ' (the lowest thing on the page: the last card, or the Recent list's
    ' last line)
    bottom = 168 + rows * pitch - (pitch - tall)
    FOR k = 0 TO 5
      IF RecentLink(k).Visible AND RecentLink(k).Top + 20 > bottom THEN bottom = RecentLink(k).Top + 20
    NEXT
    IF RecentNone.Top + 20 > bottom THEN bottom = RecentNone.Top + 20
    pageH = bottom + 24
    ' (taller than the window: laid out again beside the scroll bar)
    IF pageH <= Welcome.Height OR pass = 2 THEN EXIT FOR
    pageW = Welcome.Width - 17
  NEXT
  IF pageH < Welcome.Height THEN pageH = Welcome.Height
  WelcomePage.Width = pageW
  WelcomePage.Height = pageH
  ExamplesHead.Left = x0
  FOR k = 0 TO ExampleCount - 1
    ExCard(k).Left = x0 + (k MOD cols) * (w + 12)
    ExCard(k).Top = 168 + (k \ cols) * pitch
    ExCard(k).Width = w: ExCard(k).Height = tall
    ExTitle(k).Width = w - 56
    ExText(k).Width = w - 56
  NEXT
END SUB

' (a path shown short: the home folder as ~; a long one with its first
' folder and its last two, … between — a menu item stays readable)
FUNCTION ShortPath(Path AS STRING) AS STRING
  DIM home AS STRING, p AS STRING, n AS INTEGER
  home = ENVIRON$("HOME")
  p = REPLACESUBSTR$(Path, "\", "/")
  IF home <> "" AND LCASE$(LEFT$(p, LEN(home) + 1)) = LCASE$(home + "/") THEN p = "~" + MID$(p, LEN(home) + 1)
  n = TALLY(p, "/")
  IF LEN(p) > 40 AND n > 3 THEN
    p = FIELD$(p, "/", 1) + "/" + FIELD$(p, "/", 2) + "/…/" + FIELD$(p, "/", n) + "/" + FIELD$(p, "/", n + 1)
  END IF
  Result = p
END FUNCTION

' The Recent list's places: the Welcome page's first six links, and File >
' Open Recent's items (S2: every one, each named with its folder, "No
' Recent Projects or Files" when there are none).
SUB FillRecent
  DIM n AS INTEGER, i AS INTEGER, p AS STRING, c AS INTEGER, folder AS STRING
  n = TALLY(RecentList, CHR$(10))
  FOR i = 1 TO MAXRECENT
    c = CmdIndex("file.recent." + LTRIM$(STR$(i)))
    IF c >= 0 THEN
      IF Cmd(c).Item >= 0 THEN
        IF i <= n THEN
          p = FIELD$(RecentList, CHR$(10), i)
          folder = LEFT$(p, RINSTR(REPLACESUBSTR$(p, "\", "/"), "/") - 1)
          ' (&1 … &9, then &0: a key each, as the menus' other items)
          ' (an & in a name is shown as one, not a key)
          MI(Cmd(c).Item).Caption = "&" + LTRIM$(STR$(i MOD 10)) + "  " + REPLACESUBSTR$(FileTitle(p) + "  —  " + ShortPath(folder), "&", "&&")
          MI(Cmd(c).Item).Hint = p
          MI(Cmd(c).Item).Visible = 1
        ELSE
          MI(Cmd(c).Item).Visible = 0
        END IF
      END IF
    END IF
  NEXT
  c = CmdIndex("file.recentNone")
  IF c >= 0 THEN
    IF Cmd(c).Item >= 0 THEN MI(Cmd(c).Item).Visible = Flag(n = 0)
  END IF
  ' (and in the command palette, for the keyboard: "Open Recent")
  FOR i = 1 TO MAXRECENT
    Palette.RemoveCommand "recent:" + LTRIM$(STR$(i))
    IF i <= n THEN
      p = FIELD$(RecentList, CHR$(10), i)
      folder = LEFT$(p, RINSTR(REPLACESUBSTR$(p, "\", "/"), "/") - 1)
      Palette.AddCommand "recent:" + LTRIM$(STR$(i)), FileTitle(p) + " - " + ShortPath(folder), "", "Open Recent", "nav.history"
    END IF
  NEXT
  IF n > 6 THEN n = 6
  FOR i = 0 TO 5
    RecentLink(i).Parent = WelcomePage
    RecentLink(i).Left = 44: RecentLink(i).Top = 354 + StartShift + i * 24: RecentLink(i).Width = 230: RecentLink(i).Height = 20
    RecentLink(i).Cursor = -21
    RecentLink(i).Tag = i
    ' (links: Tab reaches each, the focus ring shows, Enter or Space opens it)
    RecentLink(i).TabStop = 1
    RecentLink(i).OnClick = RecentPicked
    IF i < n THEN
      p = FIELD$(RecentList, CHR$(10), i + 1)
      RecentLink(i).Caption = BaseName(p)
      RecentLink(i).Hint = p
      RecentLink(i).Visible = 1
    ELSE
      RecentLink(i).Visible = 0
    END IF
  NEXT
  RecentNone.Visible = Flag(n = 0)
  ' (the page as tall as its Recent list too)
  LayOutWelcome
END SUB

SUB CardPicked(Sender AS RPanel)
  OpenProject HomeDir + "examples/" + ExampleFile(Sender.Tag)
END SUB

SUB CardLabelPicked(Sender AS RLabel)
  OpenProject HomeDir + "examples/" + ExampleFile(Sender.Tag)
END SUB

SUB RecentPicked(Sender AS RLabel)
  OpenRecent Sender.Tag + 1
END SUB

' Start and Recent where they go: below the offer to reopen while it shows.
SUB PlaceStart
  DIM k AS INTEGER
  FOR k = 0 TO 4
    StartRow(k).Top = 164 + StartShift + k * 28
  NEXT
  RecentHead.Top = 324 + StartShift
  RecentNone.Top = 354 + StartShift
  FillRecent
END SUB

' (S2) The offer to reopen the last project and its files, at start-up:
' a card at the top of Start — Enter, Space or a click reopens it; the
' keyboard starts on it.
SUB ShowReopen
  DIM n AS INTEGER, name AS STRING
  ReopenPath = ReopenCandidate
  IF ReopenPath = "" THEN EXIT SUB
  n = TALLY(SessionFiles, CHR$(10))
  name = BaseName(ReopenPath)
  ReopenRow.Parent = WelcomePage
  ReopenRow.Left = 44: ReopenRow.Top = 164: ReopenRow.Width = 234: ReopenRow.Height = 52
  ReopenRow.BevelOuter = 0
  ReopenRow.Cursor = -21
  ReopenRow.TabStop = 1
  ReopenRow.OnClick = ReopenPicked
  ReopenIcon.Parent = ReopenRow
  ReopenIcon.Left = 12: ReopenIcon.Top = 14: ReopenIcon.Width = 24: ReopenIcon.Height = 24
  ReopenTitle.Parent = ReopenRow
  ReopenTitle.Left = 46: ReopenTitle.Top = 8: ReopenTitle.Width = 180: ReopenTitle.Height = 18
  ReopenTitle.Caption = "Reopen " + name
  ReopenTitle.Cursor = -21
  ReopenTitle.OnClick = ReopenLabelPicked
  ReopenText.Parent = ReopenRow
  ReopenText.Left = 46: ReopenText.Top = 28: ReopenText.Width = 180: ReopenText.Height = 18
  IF n = 1 THEN
    ReopenText.Caption = "and the file you had open"
  ELSEIF n > 1 THEN
    ReopenText.Caption = "and the " + LTRIM$(STR$(n)) + " files you had open"
  ELSE
    ReopenText.Caption = "where you left it"
  END IF
  ReopenText.Cursor = -21
  ReopenText.OnClick = ReopenLabelPicked
  ReopenRow.AccessibleName = ReopenTitle.Caption + ", " + ReopenText.Caption
  ReopenRow.Hint = ReopenPath
  ReopenRow.Visible = 1
  StartShift = 60
  ApplyReopenChrome
  PlaceStart
END SUB

SUB HideReopen
  ReopenRow.Visible = 0
  IF StartShift = 0 THEN EXIT SUB
  ReopenPath = ""
  StartShift = 0
  PlaceStart
END SUB

' (its look in the current theme: a card, as the examples')
SUB ApplyReopenChrome
  IF StartShift = 0 THEN EXIT SUB
  ReopenRow.Color = TC("card")
  ReopenTitle.Font = CardFont
  ReopenTitle.Font.Color = TC("page.text")
  ReopenText.Font = DimFont
  ReopenText.Font.Color = TC("page.dim")
  ReopenBmp.LoadIcon "nav.history", 24
  ReopenIcon.BMPHandle = ReopenBmp
END SUB

SUB ReopenPicked(Sender AS RPanel)
  ReopenLastProject
END SUB

SUB ReopenLabelPicked(Sender AS RLabel)
  ReopenLastProject
END SUB

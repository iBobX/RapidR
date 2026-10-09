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
  Dock.AddPane(ImmediateBox, "Immediate", "tab:outputbox", "debug")
  Dock.AddPane(Welcome, "Welcome", "documents", "list")
  Dock.ShowPane("outputbox")
END SUB

' ---- the start page ----------------------------------------------------------------
' Start: what to do first; Recent: the projects opened last (settings);
' Examples: RapidR's own, as cards in a grid that follows the page's width
' (each also in the command palette, for the keyboard).

DIM StartIcon(4) AS QIMAGE
DIM StartBmp(4) AS QBITMAP
DIM StartLink(4) AS QLABEL
DIM StartIconName(4) AS STRING
DIM RecentLink(6) AS QLABEL
DIM ExCard(16) AS QPANEL
DIM ExIcon(16) AS QIMAGE
DIM ExBmp(16) AS QBITMAP
DIM ExTitle(16) AS QLABEL
DIM ExText(16) AS QLABEL
DIM ExampleFile(16) AS STRING
DIM ExampleWhat(16) AS STRING
DIM ExampleCount AS INTEGER
DIM LogoBmp AS QBITMAP

SUB AddStart(k AS INTEGER, Icon AS STRING, Caption AS STRING)
  StartIcon(k).Parent = Welcome
  StartIcon(k).Left = 48: StartIcon(k).Top = 168 + k * 28: StartIcon(k).Width = 16: StartIcon(k).Height = 16
  StartIconName(k) = Icon
  StartLink(k).Parent = Welcome
  StartLink(k).Left = 74: StartLink(k).Top = 166 + k * 28: StartLink(k).Width = 200: StartLink(k).Height = 20
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
  ExCard(i).Parent = Welcome
  ExCard(i).BevelOuter = 0
  ExCard(i).Tag = i
  ExCard(i).Cursor = -21
  ExCard(i).AccessibleName = "Example " + BaseName(File) + ": " + What
  ExCard(i).OnClick = CardPicked
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

' The examples that come with RapidR (examples/, as `rapidr examples` lists them).
SUB FillExamples
  AddStart 0, "file.newProject", "New Project..."
  AddStart 1, "file.open", "Open Project or File..."
  AddStart 2, "RFORM", "Open an Example"
  AddStart 3, "view.commandPalette", "All Commands"
  ExampleCount = 0
  AddExample "gui/hello_form.rr", "A form, a button, its OnClick"
  AddExample "gui/menus.rr", "Main and pop-up menus"
  AddExample "gui/dialogs.rr", "Open, save, colour, font"
  AddExample "gui/pantry.rr", "A list and a grid together"
  AddExample "gui/themes.rr", "The themes, switched live"
  AddExample "data/csv_explorer.rr", "Drop a CSV: a table and a live chart"
  AddExample "data/dataframe.rr", "A data frame and a chart"
  AddExample "rapidq/notepad.bas", "RapidQ's notepad, as it was"
  AddExample "basics/hello.rr", "PRINT, FOR, SUB, FUNCTION"
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
  FOR k = 0 TO 3
    StartBmp(k).LoadIcon StartIconName(k), 16
    StartIcon(k).BMPHandle = StartBmp(k)
  NEXT
  FOR k = 0 TO ExampleCount - 1
    IF FileExt(ExampleFile(k)) = "bas" THEN ExBmp(k).LoadIcon "files/bas", 24 ELSE ExBmp(k).LoadIcon "files/rr", 24
    ExIcon(k).BMPHandle = ExBmp(k)
  NEXT
END SUB

' The cards in as many columns as the page has room for (200 pixels or
' more each, sharing the width), the Examples heading over them.
SUB LayOutWelcome
  DIM cols AS INTEGER, k AS INTEGER, room AS INTEGER, w AS INTEGER, x0 AS INTEGER
  x0 = 290
  room = Welcome.Width - x0 - 32
  cols = (room + 12) \ 212
  IF cols < 1 THEN cols = 1
  IF cols > 4 THEN cols = 4
  w = (room - (cols - 1) * 12) \ cols
  IF w > 300 THEN w = 300
  ExamplesHead.Left = x0
  FOR k = 0 TO ExampleCount - 1
    ExCard(k).Left = x0 + (k MOD cols) * (w + 12)
    ExCard(k).Top = 168 + (k \ cols) * 70
    ExCard(k).Width = w: ExCard(k).Height = 58
    ExTitle(k).Width = w - 56
    ExText(k).Width = w - 56
  NEXT
END SUB

SUB FillRecent
  DIM n AS INTEGER, i AS INTEGER, p AS STRING
  n = TALLY(RecentList, CHR$(10))
  IF n > 6 THEN n = 6
  FOR i = 0 TO 5
    RecentLink(i).Parent = Welcome
    RecentLink(i).Left = 48: RecentLink(i).Top = 326 + i * 24: RecentLink(i).Width = 230: RecentLink(i).Height = 20
    RecentLink(i).Cursor = -21
    RecentLink(i).Tag = i
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
END SUB

SUB CardPicked(Sender AS QPANEL)
  OpenProject HomeDir + "examples/" + ExampleFile(Sender.Tag)
END SUB

SUB CardLabelPicked(Sender AS QLABEL)
  OpenProject HomeDir + "examples/" + ExampleFile(Sender.Tag)
END SUB

SUB RecentPicked(Sender AS QLABEL)
  OpenProject FIELD$(RecentList, CHR$(10), Sender.Tag + 1)
END SUB

' panes.inc: the dock manager's panes, and the start page.
'
' Plug-in points (docs/ide-plan.md I1): each pane is registered by name with
' Dock.AddPane in SetUpDock — a wave B component replaces a pane by taking
' its place there (same name, same Where) and its Fill… SUB:
'   ProjectTree   "Project"     left              RProjectTree   (L-PANELS)
'   ToolboxTree   "Toolbox"     below Project     RToolbox       (L-PANELS)
'   PropGrid      "Properties"  right             RPropertyInspector (L-PANELS)
'   OutlineTree   "Outline"     below Properties  (RLanguageService's outline)
'   OutputBox     "Output"      bottom            ROutputConsole (L-PANELS)
'   ProblemsView  "Problems"    with Output       (RLanguageService's diagnostics)
'   ImmediateBox  "Immediate"   with Output       RImmediateWindow (I6)
'   CodeDoc(i)    a source file                   RCodeEditor    (L-EDVIEW)
'   DesignDoc(i)  a file's form                   RFormDesigner  (L-DMODEL / L-DVIEW)

SUB SetUpDock
  Dock.AddPane(ProjectTree, "Project", "left", "explorer")
  Dock.AddPane(ToolboxTree, "Toolbox", "bottom:projecttree", "toolbox")
  Dock.AddPane(PropGrid, "Properties", "right", "properties")
  Dock.AddPane(OutlineTree, "Outline", "bottom:propgrid", "outline")
  Dock.AddPane(OutputBox, "Output", "bottom:documents", "output")
  Dock.AddPane(ProblemsView, "Problems", "tab:outputbox", "problems")
  Dock.AddPane(ImmediateBox, "Immediate", "tab:outputbox", "debug")
  Dock.AddPane(Welcome, "Welcome", "documents", "list")
  Dock.ShowPane("outputbox")
END SUB

' ---- the toolbox (until RToolbox: the registry's groups, RapidQ's first) ----

SUB AddToolboxGroup(Title AS STRING, Icon AS STRING, Members AS STRING)
  DIM g AS INTEGER, i AS INTEGER, n AS INTEGER, member AS STRING
  ToolboxTree.AddItems Title
  g = ToolboxTree.ItemCount - 1
  ToolIcons.AddIcon Icon
  ToolboxTree.Item(g).ImageIndex = ToolIcons.Count - 1
  ToolboxTree.Item(g).SelectedIndex = ToolIcons.Count - 1
  n = TALLY(Members, " ") + 1
  FOR i = 1 TO n
    member = FIELD$(Members, " ", i)
    IF member <> "" THEN
      ToolboxTree.AddChildItems g, member
      ToolIcons.AddIcon member
      ToolboxTree.Item(ToolboxTree.ItemCount - 1).ImageIndex = ToolIcons.Count - 1
      ToolboxTree.Item(ToolboxTree.ItemCount - 1).SelectedIndex = ToolIcons.Count - 1
    END IF
  NEXT
END SUB

SUB FillToolbox
  ToolboxTree.Clear
  ToolIcons.Clear
  AddToolboxGroup "Standard", "groups/standard", "QFORM QBUTTON QLABEL QEDIT QCHECKBOX QRADIOBUTTON QCOMBOBOX QLISTBOX QGROUPBOX QPANEL QIMAGE QTIMER QMAINMENU QPOPUPMENU"
  AddToolboxGroup "Additional", "groups/additional", "QCOOLBTN QRICHEDIT QSTRINGGRID QLISTVIEW QTREEVIEW QTABCONTROL QTRACKBAR QGAUGE QSTATUSBAR QSPLITTER QSCROLLBOX QCANVAS"
  AddToolboxGroup "Dialogs", "groups/dialogs", "QOPENDIALOG QSAVEDIALOG QCOLORDIALOG QFONTDIALOG"
  AddToolboxGroup "Data (RapidR)", "groups/data", "RSQLITE RJSON"
  AddToolboxGroup "Data Science (RapidR)", "groups/datascience", "RDATAFRAME RPLOT"
  ToolboxTree.Expand 0, 0
END SUB

' ---- trees' icons ------------------------------------------------------------------

SUB TreeIcon(Tree AS QTREEVIEW, Index AS INTEGER, Image AS INTEGER)
  Tree.Item(Index).ImageIndex = Image
  Tree.Item(Index).SelectedIndex = Image
END SUB

' (an icon into the file icons, its index)
FUNCTION FileIcon(Name AS STRING) AS INTEGER
  FileIcons.AddIcon Name
  Result = FileIcons.Count - 1
END FUNCTION

' ---- the problems list ----------------------------------------------------------

SUB ClearProblems
  ProblemsView.Clear
  MarkIcons.Clear
  MarkIcons.AddIcon "error"
  MarkIcons.AddIcon "warning"
  MarkIcons.AddIcon "info"
  MarkIcons.AddIcon "success"
END SUB

SUB AddProblem(Kind AS INTEGER, Message AS STRING, InFile AS STRING, LineNo AS INTEGER)
  DIM i AS INTEGER
  ProblemsView.AddItems Message
  i = ProblemsView.ItemCount - 1
  ProblemsView.Item(i).ImageIndex = Kind
  ProblemsView.AddSubItem i, InFile
  IF LineNo > 0 THEN ProblemsView.AddSubItem i, STR$(LineNo) ELSE ProblemsView.AddSubItem i, ""
  IF i < 400 THEN ProblemLine(i) = LineNo
END SUB

SUB SetUpProblems
  ProblemsView.ClearColumns
  ProblemsView.AddColumns "Description", "File", "Line"
  ProblemsView.Column(0).Width = 480
  ProblemsView.Column(1).Width = 160
  ProblemsView.Column(2).Width = 60
  ClearProblems
END SUB

' ---- output ----------------------------------------------------------------------

SUB OutputLine(Text AS STRING)
  OutputBox.AddStrings Text
END SUB

' ---- properties (until RPropertyInspector: the CREATE block's assignments) ----

SUB ShowProperties(Title AS STRING)
  DIM i AS INTEGER, n AS INTEGER
  n = PropCount
  IF n < 1 THEN n = 1
  PropGrid.RowCount = n
  PropGrid.ColWidths(0) = 110
  PropGrid.ColWidths(1) = 170
  IF PropCount = 0 THEN
    PropGrid.Cell(0, 0) = "(nothing selected)"
    PropGrid.Cell(1, 0) = ""
  END IF
  FOR i = 0 TO PropCount - 1
    PropGrid.Cell(0, i) = PropKey(i)
    PropGrid.Cell(1, i) = PropValue(i)
  NEXT
  IF Title = "" THEN Dock.PaneTitle("propgrid", "Properties") ELSE Dock.PaneTitle("propgrid", "Properties: " + Title)
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

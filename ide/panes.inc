' panes.inc: the dock manager's panes and documents.
'
' Plug-in points (docs/ide-plan.md I1): each pane is registered by name with
' Dock.AddPane in SetUpDock — a wave B component replaces a pane by taking
' its place there (same name, same Where) and its Fill… SUB here:
'   ProjectTree   "Project"     left              RProjectTree   (L-PANELS)
'   ToolboxTree   "Toolbox"     below Project     RToolbox       (L-PANELS)
'   PropGrid      "Properties"  right             RPropertyInspector (L-PANELS)
'   OutlineTree   "Outline"     below Properties  (RLanguageService's outline)
'   OutputBox     "Output"      bottom            ROutputConsole (L-PANELS)
'   ProblemsView  "Problems"    with Output       (diagnostics)
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

' ---- the project tree --------------------------------------------------------

SUB TreeIcon(Tree AS QTREEVIEW, Index AS INTEGER, Image AS INTEGER)
  Tree.Item(Index).ImageIndex = Image
  Tree.Item(Index).SelectedIndex = Image
END SUB

' (an icon into the file icons, its index)
FUNCTION FileIcon(Name AS STRING) AS INTEGER
  FileIcons.AddIcon Name
  Result = FileIcons.Count - 1
END FUNCTION

SUB FillProjectTree
  DIM root AS INTEGER, k AS INTEGER, d AS INTEGER, ext AS STRING
  ProjectTree.Clear
  FileIcons.Clear
  IF ProjectOpen = 0 THEN
    ProjectTree.AddItems "No project open"
    TreeIcon ProjectTree, 0, FileIcon("files/folder")
    EXIT SUB
  END IF
  ProjectTree.AddItems ProjectName
  root = 0
  TreeIcon ProjectTree, root, FileIcon("files/rrproj")
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) AND DocKind(d) = "code" THEN
      ProjectTree.AddChildItems root, FileTitle(DocFile(d))
      k = ProjectTree.ItemCount - 1
      ext = LCASE$(MID$(DocFile(d), RINSTR(DocFile(d), ".") + 1))
      TreeIcon ProjectTree, k, FileIcon("files/" + ext)
      IF FormName <> "" THEN
        ProjectTree.AddChildItems k, FormName + " (" + FormType + ")"
        TreeIcon ProjectTree, ProjectTree.ItemCount - 1, FileIcon(FormType)
      END IF
    END IF
  NEXT
  ProjectTree.FullExpand
END SUB

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

' ---- the welcome page's examples ---------------------------------------------------

DIM ExampleFile(40) AS STRING
DIM ExampleCount AS INTEGER

SUB AddExample(File AS STRING, Kind AS STRING, What AS STRING)
  DIM i AS INTEGER
  ExampleFile(ExampleCount) = File
  ExamplesView.AddItems FileTitle(File)
  i = ExamplesView.ItemCount - 1
  IF Kind = "gui" THEN ExamplesView.Item(i).ImageIndex = 0 ELSE ExamplesView.Item(i).ImageIndex = 1
  ExamplesView.AddSubItem i, LEFT$(File, INSTR(File, "/") - 1)
  ExamplesView.AddSubItem i, What
  ExampleCount = ExampleCount + 1
END SUB

' The examples that come with RapidR (examples/, as `rapidr examples` lists them).
SUB FillExamples
  ExamplesView.Clear
  ExamplesView.ClearColumns
  ExamplesView.AddColumns "Example", "Topic", "What it shows"
  ExamplesView.Column(0).Width = 120
  ExamplesView.Column(1).Width = 70
  ExamplesView.Column(2).Width = 400
  ExampleIcons.Clear
  ExampleIcons.AddIcon "QFORM"
  ExampleIcons.AddIcon "files/rr"
  ExampleCount = 0
  AddExample "gui/hello_form.rr", "gui", "A form, a button and its OnClick"
  AddExample "gui/menus.rr", "gui", "A main menu, a pop-up menu, shortcuts"
  AddExample "gui/dialogs.rr", "gui", "Open, save, colour and font dialogs"
  AddExample "gui/stopwatch.rr", "gui", "A QTIMER driving a display"
  AddExample "gui/pantry.rr", "gui", "A list and a grid working together"
  AddExample "gui/themes.rr", "gui", "The four themes, switched live"
  AddExample "graphics/canvas.rr", "gui", "Drawing on a QCANVAS"
  AddExample "data/dataframe.rr", "gui", "A data frame and a chart"
  AddExample "rapidq/notepad.bas", "gui", "A RapidQ notepad, as RapidQ wrote it"
  AddExample "basics/hello.rr", "console", "PRINT, variables, FOR, SUB and FUNCTION"
  AddExample "basics/language.rr", "console", "TYPE, arrays, DATA, SELECT CASE, objects"
  AddExample "basics/files.rr", "console", "Text files and streams"
  AddExample "data/sqlite.rr", "console", "A SQLite database"
  AddExample "data/json.rr", "console", "Reading and writing JSON"
END SUB

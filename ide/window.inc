' window.inc: RapidR Studio's main window — the menu bar (filled from the
' command table), the tool bar, the dock manager with its panes and the
' documents area, the status bar — and the command palette.
'
' Every pane is a public component registered in the dock manager by name,
' so a richer component replaces one by taking its name (docs/ide-plan.md
' I1): ProjectTree (RProjectTree), ToolboxTree (RToolbox), PropGrid
' (RPropertyInspector), OutlineTree, OutputBox (ROutputConsole),
' ProblemsView, ImmediateBox (RImmediateWindow); documents are CodeDoc(i)
' (RCodeEditor) and DesignDoc(i) (RFormDesigner) — see panes.inc.

DIM FileIcons AS QIMAGELIST
FileIcons.Width = 16: FileIcons.Height = 16
DIM ToolIcons AS QIMAGELIST
ToolIcons.Width = 16: ToolIcons.Height = 16
DIM SymbolIcons AS QIMAGELIST
SymbolIcons.Width = 16: SymbolIcons.Height = 16
DIM MarkIcons AS QIMAGELIST
MarkIcons.Width = 16: MarkIcons.Height = 16
DIM ExampleIcons AS QIMAGELIST
ExampleIcons.Width = 16: ExampleIcons.Height = 16

DIM HeadFont AS QFONT
HeadFont.Size = 20: HeadFont.Bold = 1
DIM SubFont AS QFONT
SubFont.Size = 11: SubFont.Bold = 1
DIM LinkFont AS QFONT
LinkFont.Size = 9
DIM MonoFont AS QFONT
MonoFont.Name = "Courier New": MonoFont.Size = 10

CREATE Studio AS QFORM
  Caption = "RapidR Studio"
  Width = 1280: Height = 800
  KeyPreview = 1
  OnResize = StudioResize
  OnClose = StudioClose
  CREATE MainMenu AS QMAINMENU
  END CREATE
  CREATE ToolBar AS QPANEL
    Align = 1: Height = 38
    BevelOuter = 0
    AccessibleName = "Tool bar"
  END CREATE
  CREATE ToolLine AS QPANEL
    Align = 1: Height = 1
    BevelOuter = 0
  END CREATE
  CREATE Status AS QSTATUSBAR
    AccessibleName = "Status"
  END CREATE
  CREATE Dock AS RDOCKMANAGER
    Align = 5
    OnPaneChange = PaneChanged
    OnDocumentActivate = DocumentActivated
    OnDocumentClose = DocumentClosing
  END CREATE

  ' ---- tool panes ----
  CREATE ProjectTree AS QTREEVIEW
    Width = 250: Height = 300
    Images = FileIcons
    AccessibleName = "Project"
    OnDblClick = ProjectTreeOpen
  END CREATE
  CREATE ToolboxTree AS QTREEVIEW
    Width = 250: Height = 300
    Images = ToolIcons
    AccessibleName = "Toolbox"
  END CREATE
  CREATE PropGrid AS QSTRINGGRID
    Width = 290: Height = 340
    ColCount = 2: RowCount = 2
    FixedCols = 1: FixedRows = 0
    DefaultRowHeight = 22
    AccessibleName = "Properties"
  END CREATE
  CREATE OutlineTree AS QTREEVIEW
    Width = 290: Height = 260
    Images = SymbolIcons
    AccessibleName = "Outline"
    OnDblClick = OutlineOpen
  END CREATE
  CREATE OutputBox AS QRICHEDIT
    Width = 700: Height = 190
    ReadOnly = 1
    WordWrap = 0
    AccessibleName = "Output"
  END CREATE
  CREATE ProblemsView AS QLISTVIEW
    Width = 700: Height = 190
    ViewStyle = 3
    RowSelect = 1
    ReadOnly = 1
    SmallImages = MarkIcons
    AccessibleName = "Problems"
    OnDblClick = ProblemOpen
  END CREATE
  CREATE ImmediateBox AS QRICHEDIT
    Width = 700: Height = 190
    WordWrap = 0
    AccessibleName = "Immediate"
  END CREATE

  ' ---- the start page (a document) ----
  CREATE Welcome AS QPANEL
    Width = 720: Height = 520
    BevelOuter = 0
    AccessibleName = "Welcome"
    CREATE WelcomeTitle AS QLABEL
      Left = 40: Top = 30: Width = 600: Height = 36
      Caption = "RapidR Studio"
    END CREATE
    CREATE WelcomeTag AS QLABEL
      Left = 40: Top = 70: Width = 640: Height = 20
      Caption = "RapidQ programs, made modern: desktop and web from one source."
    END CREATE
    CREATE StartHead AS QLABEL
      Left = 40: Top = 112: Width = 200: Height = 22
      Caption = "Start"
    END CREATE
    CREATE LinkNew AS QLABEL
      Left = 40: Top = 142: Width = 220: Height = 20
      Caption = "New Project..."
      Cursor = -21
      OnClick = WelcomeLink
    END CREATE
    CREATE LinkOpen AS QLABEL
      Left = 40: Top = 166: Width = 220: Height = 20
      Caption = "Open Project or File..."
      Cursor = -21
      OnClick = WelcomeLink
    END CREATE
    CREATE LinkExample AS QLABEL
      Left = 40: Top = 190: Width = 220: Height = 20
      Caption = "Open an Example..."
      Cursor = -21
      OnClick = WelcomeLink
    END CREATE
    CREATE LinkPalette AS QLABEL
      Left = 40: Top = 214: Width = 220: Height = 20
      Caption = "All Commands (Ctrl+Shift+P)"
      Cursor = -21
      OnClick = WelcomeLink
    END CREATE
    CREATE RecentHead AS QLABEL
      Left = 290: Top = 112: Width = 300: Height = 22
      Caption = "Examples"
    END CREATE
    CREATE ExamplesView AS QLISTVIEW
      Left = 290: Top = 142: Width = 400: Height = 340
      Anchors = akLeft + akTop + akRight + akBottom
      ViewStyle = 3
      RowSelect = 1
      ReadOnly = 1
      SmallImages = ExampleIcons
      AccessibleName = "Examples"
      OnDblClick = ExampleOpened
    END CREATE
  END CREATE
END CREATE

' ---- the command palette (Ctrl+Shift+P) -----------------------------------------
CREATE Palette AS QFORM
  Caption = "Command Palette"
  Width = 560: Height = 360
  BorderStyle = 4
  KeyPreview = 1
  OnKeyDown = PaletteKey
  CREATE PaletteEdit AS QEDIT
    Left = 8: Top = 8: Width = 528: Height = 24
    AccessibleName = "Type a command"
    OnChange = PaletteChanged
  END CREATE
  CREATE PaletteList AS QLISTBOX
    Left = 8: Top = 40: Width = 528: Height = 276
    AccessibleName = "Commands"
    OnDblClick = PalettePicked
  END CREATE
END CREATE

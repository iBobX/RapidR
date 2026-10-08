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

' The fonts: RapidR's look's chrome in Inter, code in JetBrains Mono
' (docs/ide-plan.md D8); the classic look keeps Windows' (RapidQ's).
DIM UIFont AS QFONT
DIM HeadFont AS QFONT
DIM SubFont AS QFONT
DIM CardFont AS QFONT
DIM DimFont AS QFONT
DIM LinkFont AS QFONT
DIM StatusFont AS QFONT
DIM MonoFont AS QFONT

CREATE Studio AS QFORM
  Caption = "RapidR Studio"
  Width = 1280: Height = 800
  KeyPreview = 1
  OnResize = StudioResize
  OnClose = StudioClose
  OnKeyDown = StudioKey
  CREATE MainMenu AS QMAINMENU
  END CREATE
  CREATE ToolBar AS QPANEL
    Align = 1: Height = 40
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
    OnKeyDown = ImmediateKey
  END CREATE
  ' (the problems and the outline follow the editor once typing stops)
  CREATE Macro AS QTIMER
    Interval = 400
    Enabled = 0
    OnTimer = MacroTick
  END CREATE
  ' (editor.inc: a designer action's source patches end with the turn)
  CREATE PatchBurst AS QTIMER
    Interval = 1
    Enabled = 0
    OnTimer = PatchBurstTick
  END CREATE
  CREATE Analyzer AS QTIMER
    Interval = 600
    Enabled = 0
    OnTimer = AnalyzerTick
  END CREATE

  ' ---- the start page (a document): the brand, Start, Recent, the examples ----
  CREATE Welcome AS QPANEL
    Width = 720: Height = 520
    BevelOuter = 0
    AccessibleName = "Welcome"
    OnResize = LayOutWelcome
    CREATE WelcomeLogo AS QIMAGE
      Left = 48: Top = 36: Width = 153: Height = 44
      Stretch = 1
      AccessibleName = "RapidR"
    END CREATE
    CREATE WelcomeTitle AS QLABEL
      Left = 216: Top = 44: Width = 400: Height = 30
      Caption = "Studio"
    END CREATE
    CREATE WelcomeTag AS QLABEL
      Left = 48: Top = 92: Width = 640: Height = 20
      Caption = "RapidQ programs, made modern: desktop and web from one source."
    END CREATE
    CREATE StartHead AS QLABEL
      Left = 48: Top = 136: Width = 220: Height = 22
      Caption = "Start"
    END CREATE
    CREATE RecentHead AS QLABEL
      Left = 48: Top = 296: Width = 220: Height = 22
      Caption = "Recent"
    END CREATE
    CREATE RecentNone AS QLABEL
      Left = 48: Top = 326: Width = 220: Height = 20
      Caption = "No recent projects yet"
    END CREATE
    CREATE ExamplesHead AS QLABEL
      Left = 300: Top = 136: Width = 300: Height = 22
      Caption = "Examples"
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

' ---- File > New Project ---------------------------------------------------------------
CREATE NewDialog AS QFORM
  Caption = "New Project"
  Width = 460: Height = 230
  BorderStyle = 3
  CREATE NewNameLabel AS QLABEL
    Left = 16: Top = 20: Width = 90: Caption = "&Name:"
  END CREATE
  CREATE NewName AS QEDIT
    Left = 110: Top = 16: Width = 320
  END CREATE
  CREATE NewKindLabel AS QLABEL
    Left = 16: Top = 56: Width = 90: Caption = "&Kind:"
  END CREATE
  CREATE NewKind AS QCOMBOBOX
    Left = 110: Top = 52: Width = 320
    AddItems "A program with a form", "A console program"
  END CREATE
  CREATE NewFolderLabel AS QLABEL
    Left = 16: Top = 92: Width = 90: Caption = "&Folder:"
  END CREATE
  CREATE NewFolder AS QEDIT
    Left = 110: Top = 88: Width = 284
  END CREATE
  CREATE NewFolderBtn AS QBUTTON
    Left = 400: Top = 87: Width = 30: Caption = "..."
    AccessibleName = "Choose the folder"
    OnClick = NewBrowse
  END CREATE
  CREATE NewOk AS QBUTTON
    Left = 254: Top = 140: Width = 84: Caption = "Create"
    Default = 1
    OnClick = NewCreate
  END CREATE
  CREATE NewNo AS QBUTTON
    Left = 346: Top = 140: Width = 84: Caption = "Cancel"
    Cancel = 1
    OnClick = NewCancel
  END CREATE
END CREATE

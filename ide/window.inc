' window.inc: RapidR Studio's main window — the menu bar (filled from the
' command table), the tool bar, the dock manager with its panes and the
' documents area, the status bar — and the command palette.
'
' Every pane is a public component registered in the dock manager by name
' (docs/ide-plan.md I1): ProjectTree (RPROJECTTREE), Toolbox (RTOOLBOX),
' Inspector (RPROPERTYINSPECTOR), OutlineTree, OutputBox (ROUTPUTCONSOLE:
' the output, the build log and the problems), ImmediateBox
' (RImmediateWindow, I6); the tool bar is an RTOOLBAR and the command
' palette an RCOMMANDPALETTE (panels.inc); documents are CodeDoc(i)
' (RCodeEditor) and DesignDoc(i) (the designer) — see panes.inc.

DIM SymbolIcons AS QIMAGELIST
SymbolIcons.Width = 16: SymbolIcons.Height = 16
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
  CREATE ToolBar AS RTOOLBAR
    Align = 1: Height = 40
    ButtonSize = 30
    AccessibleName = "Tool bar"
    OnButtonClick = ToolClicked
  END CREATE
  CREATE Status AS QSTATUSBAR
    AccessibleName = "Status"
  END CREATE
  CREATE Dock AS RDOCKMANAGER
    Align = 5
    DocumentMode = "tabs"
    OnPaneChange = PaneChanged
    OnDocumentActivate = DocumentActivated
    OnDocumentClose = DocumentClosing
  END CREATE

  ' ---- tool panes ----
  CREATE ProjectTree AS RPROJECTTREE
    Width = 250: Height = 300
    AccessibleName = "Project"
    OnOpen = ProjectNodeOpen
    OnSelect = ProjectNodeSelect
    OnNewFile = ProjectNewFile
    OnRename = ProjectRename
    OnDelete = ProjectDelete
    OnMove = ProjectMove
  END CREATE
  CREATE Toolbox AS RTOOLBOX
    Width = 250: Height = 300
    AccessibleName = "Toolbox"
    OnPick = ToolboxPick
    OnSelect = ToolboxSelect
    OnDragStart = ToolboxDragStart
    OnDragDrop = ToolboxDrop
  END CREATE
  CREATE Inspector AS RPROPERTYINSPECTOR
    Width = 290: Height = 340
    AccessibleName = "Properties"
    OnPropertyChange = InspectorChanged
    OnEventDblClick = InspectorEvent
  END CREATE
  CREATE OutlineTree AS QTREEVIEW
    Width = 290: Height = 260
    Images = SymbolIcons
    AccessibleName = "Outline"
    OnDblClick = OutlineOpen
  END CREATE
  CREATE OutputBox AS ROUTPUTCONSOLE
    Width = 700: Height = 190
    AccessibleName = "Output"
    OnLinkClick = OutputLink
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
  CREATE EditorFocus AS QTIMER
    Interval = 1
    Enabled = 0
    OnTimer = EditorFocusTick
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

  ' ---- the command palette (Ctrl+Shift+P): over the window's top ----
  CREATE Palette AS RCOMMANDPALETTE
    Width = 600
    Placeholder = "Type a command, :line, or an example"
    AccessibleName = "Command Palette"
    OnCommand = PaletteCommand
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
    Left = 16: Top = 56: Width = 90: Caption = "Template:"
  END CREATE
  ' (the templates' cards: workspace.inc's gallery)
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

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

DIM SymbolIcons AS RImageList
SymbolIcons.Width = 16: SymbolIcons.Height = 16
DIM ExampleIcons AS RImageList
ExampleIcons.Width = 16: ExampleIcons.Height = 16

' The fonts: RapidR's look's chrome in Inter, code in JetBrains Mono
' (docs/ide-plan.md D8); the classic look keeps Windows' (RapidQ's).
DIM UIFont AS RFont
DIM HeadFont AS RFont
DIM SubFont AS RFont
DIM CardFont AS RFont
DIM DimFont AS RFont
DIM LinkFont AS RFont
DIM StatusFont AS RFont
DIM MonoFont AS RFont

CREATE Studio AS RForm
  Caption = "RapidR Studio"
  Width = 1280: Height = 800
  KeyPreview = 1
  OnResize = StudioResize
  OnClose = StudioClose
  OnKeyDown = StudioKey
  CREATE MainMenu AS RMainMenu
  END CREATE
  CREATE ToolBar AS RToolBar
    Align = 1: Height = 40
    ButtonSize = 30
    AccessibleName = "Tool bar"
    OnButtonClick = ToolClicked
  END CREATE
  CREATE Status AS RStatusBar
    AccessibleName = "Status"
  END CREATE
  CREATE Dock AS RDockManager
    Align = 5
    DocumentMode = "tabs"
    OnPaneChange = PaneChanged
    OnDocumentActivate = DocumentActivated
    OnDocumentClose = DocumentClosing
  END CREATE

  ' ---- tool panes ----
  CREATE ProjectTree AS RProjectTree
    Width = 250: Height = 300
    AccessibleName = "Project"
    OnOpen = ProjectNodeOpen
    OnSelect = ProjectNodeSelect
    OnNewFile = ProjectNewFile
    OnRename = ProjectRename
    OnDelete = ProjectDelete
    OnMove = ProjectMove
  END CREATE
  CREATE Toolbox AS RToolbox
    Width = 250: Height = 300
    AccessibleName = "Toolbox"
    OnPick = ToolboxPick
    OnSelect = ToolboxSelect
    OnDragStart = ToolboxDragStart
    OnDragDrop = ToolboxDrop
  END CREATE
  CREATE Inspector AS RPropertyInspector
    Width = 290: Height = 340
    AccessibleName = "Properties"
    OnPropertyChange = InspectorChanged
    OnEventDblClick = InspectorEvent
  END CREATE
  CREATE OutlineTree AS RTreeView
    Width = 290: Height = 260
    Images = SymbolIcons
    AccessibleName = "Outline"
    OnDblClick = OutlineOpen
  END CREATE
  CREATE OutputBox AS ROutputConsole
    Width = 700: Height = 190
    AccessibleName = "Output"
    OnLinkClick = OutputLink
  END CREATE
  CREATE ImmediateBox AS RRichEdit
    Width = 700: Height = 190
    WordWrap = 0
    AccessibleName = "Immediate"
    OnKeyDown = ImmediateKey
  END CREATE
  ' (the problems and the outline follow the editor once typing stops)
  CREATE Macro AS RTimer
    Interval = 400
    Enabled = 0
    OnTimer = MacroTick
  END CREATE
  CREATE Analyzer AS RTimer
    Interval = 600
    Enabled = 0
    OnTimer = AnalyzerTick
  END CREATE

  ' ---- the start page (a document): the brand, Start, Recent, the examples ----
  CREATE Welcome AS RPanel
    Width = 720: Height = 520
    BevelOuter = 0
    AccessibleName = "Welcome"
    OnResize = LayOutWelcome
    CREATE WelcomeLogo AS RImage
      Left = 48: Top = 36: Width = 153: Height = 44
      Stretch = 1
      AccessibleName = "RapidR"
    END CREATE
    CREATE WelcomeTitle AS RLabel
      Left = 216: Top = 44: Width = 400: Height = 30
      Caption = "Studio"
    END CREATE
    CREATE WelcomeTag AS RLabel
      Left = 48: Top = 92: Width = 640: Height = 20
      Caption = "Runs RapidQ programs, and takes them further: desktop and web from one source."
    END CREATE
    CREATE StartHead AS RLabel
      Left = 48: Top = 136: Width = 220: Height = 22
      Caption = "Start"
    END CREATE
    CREATE RecentHead AS RLabel
      Left = 48: Top = 324: Width = 220: Height = 22
      Caption = "Recent"
    END CREATE
    CREATE RecentNone AS RLabel
      Left = 48: Top = 354: Width = 220: Height = 20
      Caption = "No recent projects yet"
    END CREATE
    CREATE ExamplesHead AS RLabel
      Left = 300: Top = 136: Width = 300: Height = 22
      Caption = "Examples"
    END CREATE
  END CREATE

  ' ---- the command palette (Ctrl+Shift+P): over the window's top ----
  CREATE Palette AS RCommandPalette
    Width = 600
    Placeholder = "Type a command, :line, or an example"
    AccessibleName = "Command Palette"
    OnCommand = PaletteCommand
  END CREATE
END CREATE


' ---- File > New Project ---------------------------------------------------------------
CREATE NewDialog AS RForm
  Caption = "New Project"
  Width = 460: Height = 230
  BorderStyle = 3
  CREATE NewNameLabel AS RLabel
    Left = 16: Top = 20: Width = 90: Caption = "&Name:"
  END CREATE
  CREATE NewName AS REdit
    Left = 110: Top = 16: Width = 320
  END CREATE
  CREATE NewKindLabel AS RLabel
    Left = 16: Top = 56: Width = 90: Caption = "Template:"
  END CREATE
  ' (the templates' cards: workspace.inc's gallery)
  CREATE NewFolderLabel AS RLabel
    Left = 16: Top = 92: Width = 90: Caption = "&Folder:"
  END CREATE
  CREATE NewFolder AS REdit
    Left = 110: Top = 88: Width = 284
  END CREATE
  CREATE NewFolderBtn AS RButton
    Left = 400: Top = 87: Width = 30: Caption = "..."
    AccessibleName = "Choose the folder"
    OnClick = NewBrowse
  END CREATE
  CREATE NewOk AS RButton
    Left = 254: Top = 140: Width = 84: Caption = "Create"
    Default = 1
    OnClick = NewCreate
  END CREATE
  CREATE NewNo AS RButton
    Left = 346: Top = 140: Width = 84: Caption = "Cancel"
    Cancel = 1
    OnClick = NewCancel
  END CREATE
END CREATE

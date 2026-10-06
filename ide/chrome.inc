' chrome.inc: Studio's look in the current theme — the tool bar, the status
' bar and its state, the start page — from the theme's colours
' (Application.ThemeColor: the theme's tokens and the IDE's, rapidr_value's
' ide_theme), applied again when the theme changes.

' (a theme colour, as a RapidQ colour)
FUNCTION TC(Name AS STRING) AS LONG
  Result = Application.ThemeColor(Name)
END FUNCTION

SUB ApplyChrome
  DIM k AS INTEGER
  ThemeName = Application.Theme
  ToolBar.Color = TC("toolbar")
  ToolLine.Color = TC("toolbar.border")
  FOR k = 0 TO TBSepCount - 1
    TBSep(k).Color = TC("separator")
  NEXT
  LoadToolIcons
  ' (the start page)
  Welcome.Color = TC("page")
  WelcomeTitle.Font = HeadFont
  WelcomeTitle.Font.Color = TC("page.heading")
  WelcomeTag.Font.Color = TC("page.dim")
  StartHead.Font = SubFont
  StartHead.Font.Color = TC("page.heading")
  RecentHead.Font = SubFont
  RecentHead.Font.Color = TC("page.heading")
  LinkNew.Font.Color = TC("page.link")
  LinkOpen.Font.Color = TC("page.link")
  LinkExample.Font.Color = TC("page.link")
  LinkPalette.Font.Color = TC("page.link")
  OutputBox.Font = MonoFont
  ImmediateBox.Font = MonoFont
  ApplyState
END SUB

' The status bar: what the program under development is doing, in the
' state's colour (classic and high contrast keep their bar: the words say it).
SUB ApplyState
  SELECT CASE RunState
    CASE "running"
      Status.Color = TC("statusbar.running")
      Status.Font.Color = TC("statusbar.statetext")
    CASE "paused"
      Status.Color = TC("statusbar.debugging")
      Status.Font.Color = TC("statusbar.statetext")
    CASE "error"
      Status.Color = TC("statusbar.error")
      Status.Font.Color = TC("statusbar.statetext")
    CASE ELSE
      Status.Color = TC("statusbar")
      Status.Font.Color = TC("statusbar.text")
  END SELECT
  Status.Panel(0).Caption = "  " + StateText
END SUB

SUB SetState(State AS STRING, Text AS STRING)
  RunState = State
  StateText = Text
  ApplyState
  UpdateCommands
END SUB

SUB SetUpStatus
  Status.AddPanels "", "", "Ln 1, Col 1", "UTF-8", "CRLF", "RapidQ BASIC"
  LayOutStatus
END SUB

' (the state at the left, the file's facts at the right; the project between)
SUB LayOutStatus
  DIM w AS INTEGER
  w = Studio.ClientWidth - 230 - 110 - 70 - 60 - 160
  IF w < 40 THEN w = 40
  Status.Panel(0).Width = 230
  Status.Panel(1).Width = w
  Status.Panel(2).Width = 110
  Status.Panel(3).Width = 70
  Status.Panel(4).Width = 60
  IF ProjectOpen THEN Status.Panel(1).Caption = ProjectName ELSE Status.Panel(1).Caption = ""
END SUB

SUB UpdateTitle
  IF ProjectOpen THEN Studio.Caption = ProjectName + " - RapidR Studio" ELSE Studio.Caption = "RapidR Studio"
  LayOutStatus
END SUB

SUB SetTheme(Name AS STRING)
  ThemeChoice = Name
  Application.Theme = Name
  ApplyChrome
  FillToolbox
  FillProjectTree
  ScanActive
  UpdateCommands
END SUB

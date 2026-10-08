' chrome.inc: Studio's look in the current theme — the tool bar, the status
' bar and its state, the start page — from the theme's colours
' (Application.ThemeColor: the theme's tokens and the IDE's, rapidr_value's
' ide_theme), applied again when the theme changes.

' (a theme colour, as a RapidQ colour)
FUNCTION TC(Name AS STRING) AS LONG
  Result = Application.ThemeColor(Name)
END FUNCTION

' The fonts of the look: Inter / JetBrains Mono in RapidR's, Windows' in
' the classic one; trees' and lists' rows roomier in RapidR's.
SUB ApplyFonts
  DIM k AS INTEGER, rows AS INTEGER
  IF ThemeName = "classic" THEN
    UIFont.Name = "MS Sans Serif": UIFont.Size = 8
    MonoFont.Name = "Courier New": MonoFont.Size = 10
    StatusFont.Name = "MS Sans Serif": StatusFont.Size = 8
    HeadFont.Name = "Arial": HeadFont.Size = 20: HeadFont.Bold = 1
    SubFont.Name = "Arial": SubFont.Size = 11: SubFont.Bold = 1
    CardFont.Name = "MS Sans Serif": CardFont.Size = 8: CardFont.Bold = 1
    DimFont.Name = "MS Sans Serif": DimFont.Size = 8
    rows = 16
  ELSE
    UIFont.Name = "Inter": UIFont.Size = 10
    MonoFont.Name = "JetBrains Mono": MonoFont.Size = 10
    StatusFont.Name = "Inter": StatusFont.Size = 9
    HeadFont.Name = "Inter": HeadFont.Size = 20: HeadFont.Bold = 1
    SubFont.Name = "Inter": SubFont.Size = 11: SubFont.Bold = 1
    CardFont.Name = "Inter": CardFont.Size = 10: CardFont.Bold = 1
    DimFont.Name = "Inter": DimFont.Size = 9
    rows = 22
  END IF
  LinkFont.Name = UIFont.Name: LinkFont.Size = UIFont.Size
  OutlineTree.Font = UIFont: OutlineTree.ItemHeight = rows: OutlineTree.Indent = 16
  ApplyPanelFonts
  Dock.Font = UIFont
  Status.Font = StatusFont
  ' (Studio's dialogs: the chrome font's face and size on the form, which
  ' their components follow — Delphi's ParentFont; not a QFONT, whose
  ' colour would be chosen for them and win over the theme's text colour;
  ' sizes to match)
  NewDialog.Font.Name = UIFont.Name: NewDialog.Font.Size = UIFont.Size
  LayOutDialogs
  ImmediateBox.Font = MonoFont
  FOR k = 0 TO DocCount - 1
    CodeDoc(k).Font = MonoFont
  NEXT
  WelcomeTag.Font = UIFont
  RecentNone.Font = DimFont
  FOR k = 0 TO 3
    StartLink(k).Font = LinkFont
  NEXT
  FOR k = 0 TO 5
    RecentLink(k).Font = LinkFont
  NEXT
  FOR k = 0 TO ExampleCount - 1
    ExTitle(k).Font = CardFont
    ExText(k).Font = DimFont
  NEXT
END SUB

' Studio's dialogs' controls: RapidQ's sizes in the classic look, the
' RAPIDR look's (Inter 13 px: 28 px fields, 32 px buttons, more air) in the
' others.
SUB LayOutDialogs
  DIM row AS INTEGER, fh AS INTEGER, bh AS INTEGER, bw AS INTEGER, x AS INTEGER, w AS INTEGER, y AS INTEGER, pad AS INTEGER
  IF ThemeName = "classic" THEN
    row = 36: fh = 21: bh = 25: bw = 84: x = 110: w = 320: pad = 16
  ELSE
    row = 44: fh = 28: bh = 32: bw = 96: x = 112: w = 340: pad = 20
  END IF
  y = pad
  NewNameLabel.Left = pad: NewNameLabel.Top = y + (fh - 16) / 2
  NewName.Left = x: NewName.Top = y: NewName.Width = w: NewName.Height = fh
  y = y + row
  NewKindLabel.Left = pad: NewKindLabel.Top = y + (fh - 16) / 2
  NewKind.Left = x: NewKind.Top = y: NewKind.Width = w
  y = y + row
  NewFolderLabel.Left = pad: NewFolderLabel.Top = y + (fh - 16) / 2
  NewFolder.Left = x: NewFolder.Top = y: NewFolder.Width = w - fh - 8: NewFolder.Height = fh
  NewFolderBtn.Left = x + w - fh: NewFolderBtn.Top = y: NewFolderBtn.Width = fh: NewFolderBtn.Height = fh
  y = y + row + pad / 2
  NewNo.Width = bw: NewNo.Height = bh: NewNo.Left = x + w - bw: NewNo.Top = y
  NewOk.Width = bw: NewOk.Height = bh: NewOk.Left = x + w - 2 * bw - 8: NewOk.Top = y
  NewDialog.ClientWidth = x + w + pad
  NewDialog.ClientHeight = y + bh + pad
END SUB

SUB ApplyChrome
  DIM k AS INTEGER
  ThemeName = Application.Theme
  ApplyFonts
  ' (the start page)
  Welcome.Color = TC("page")
  WelcomeTitle.Font = HeadFont
  WelcomeTitle.Font.Color = TC("page.dim")
  WelcomeTag.Font.Color = TC("page.dim")
  StartHead.Font = SubFont
  StartHead.Font.Color = TC("page.heading")
  RecentHead.Font = SubFont
  RecentHead.Font.Color = TC("page.heading")
  ExamplesHead.Font = SubFont
  ExamplesHead.Font.Color = TC("page.heading")
  RecentNone.Font.Color = TC("page.dim")
  FOR k = 0 TO 3
    StartLink(k).Font.Color = TC("page.link")
  NEXT
  FOR k = 0 TO 5
    RecentLink(k).Font.Color = TC("page.link")
  NEXT
  FOR k = 0 TO ExampleCount - 1
    ExCard(k).Color = TC("card")
    ExTitle(k).Font = CardFont
    ExTitle(k).Font.Color = TC("page.text")
    ExText(k).Font.Color = TC("page.dim")
  NEXT
  LoadWelcomeIcons
  ImmediateBox.Font = MonoFont
  ApplySearchChrome
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
  FillProjectTree
  AnalyzeActive
  UpdateCommands
END SUB

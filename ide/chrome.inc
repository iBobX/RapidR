' chrome.inc: Studio's look in the current theme — the tool bar, the status
' bar and its state, the start page — from the theme's colours
' (Application.ThemeColor: the theme's tokens and the IDE's, rapidr_value's
' ide_theme), applied again when the theme changes.

' (a theme colour, as a RapidQ colour)
FUNCTION TC(Name AS STRING) AS LONG
  Result = Application.ThemeColor(Name)
END FUNCTION

' ---- the design guide's sizes and chips (docs/design-guidelines.md) ----------------

' A compact control's height: everything in a pane's header, a tool strip
' or an inline question — buttons, combo boxes, fields, chips — is this
' tall, on one centre line. It is the height RapidR gives a combo box and a
' field in the chrome font (its line and 8: 24 in Inter 10 pt, 21 in MS
' Sans Serif 8), so a button beside them matches.
FUNCTION UiCompactH AS INTEGER
  IF ThemeName = "classic" THEN Result = 21 ELSE Result = 24
END FUNCTION

' A dialog's command button's height (OK, Cancel, Apply, Copy Address …).
FUNCTION UiButtonH AS INTEGER
  IF ThemeName = "classic" THEN Result = 25 ELSE Result = 32
END FUNCTION

' A pane's header with a title and a one-line subtitle under it.
FUNCTION UiPaneHeadH AS INTEGER
  IF ThemeName = "classic" THEN Result = 44 ELSE Result = 52
END FUNCTION

' The status a chip's Tag says: 0 info, 1 success, 2 warning, 3 danger.
FUNCTION ChipStatus(Tag AS INTEGER) AS STRING
  SELECT CASE Tag
    CASE 1: Result = "success"
    CASE 2: Result = "warning"
    CASE 3: Result = "danger"
    CASE ELSE: Result = "info"
  END SELECT
END FUNCTION

' A chip (a status pill): an RCanvas whose Tag is its status and whose
' Color is the ground it sits on, OnPaint = ChipPaint; its icon (an RImage,
' 16 px, from RapidR's icons) and its text (an RLabel, Transparent, in
' status.<name>.text) on top. A soft fill, a 1-pixel border, round ends —
' never a solid block of colour.
SUB ChipPaint(Sender AS RCanvas)
  DIM k AS STRING, w AS INTEGER, h AS INTEGER
  k = "status." + ChipStatus(Sender.Tag)
  w = Sender.Width: h = Sender.Height
  Sender.FillRect 0, 0, w, h, Sender.Color
  Sender.RoundRect 0, 0, w, h, h, h, TC(k + ".border")
  Sender.RoundRect 1, 1, w - 1, h - 1, h - 2, h - 2, TC(k)
END SUB

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
  ApplyDebugFonts rows
  ApplyPanelFonts
  Dock.Font = UIFont
  Status.Font = StatusFont
  McpFonts
  ' (Studio's dialogs: the chrome font's face and size on the form, which
  ' their components follow — inherited from the parent; not a QFONT, whose
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
  FOR k = 0 TO 4
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
  ' (the templates' cards in a row under their heading: workspace.inc)
  NewKindLabel.Left = pad: NewKindLabel.Top = y
  y = y + 22
  LayOutTemplates pad, y
  y = y + TPL_H + pad
  IF TemplatesWidth + 2 * pad > x + w + pad THEN w = TemplatesWidth + pad - x
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
  WelcomePage.Color = TC("page")
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
  FOR k = 0 TO 4
    StartLink(k).Font.Color = TC("page.link")
    StartRow(k).Color = TC("page")
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
  ' (S2: the offer to reopen the last project, a card as the examples)
  ApplyReopenChrome
  ImmediateBox.Font = MonoFont
  ApplyDebugColors
  ApplySearchChrome
  ApplyVersionsChrome
  ApplyExternalChrome
  ApplyCrashChrome
  ' (School Lock's card, chip and strips: schoollock.inc)
  SchoolLockChrome
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
  ' (the state takes the room it needs: LayOutStatus)
  LayOutStatus
END SUB

' (the bar's tooltip and what a screen reader says beside its panels: what
' a shortened panel says in full)
SUB ShowStatusHint
  DIM h AS STRING
  IF Status.Panel(0).Caption <> TR$(StateText) THEN h = TR$(StateText)
  IF ProjectOpen AND Status.Panel(1).Caption <> ProjectName + VerStatusText THEN
    IF h <> "" THEN h = h + " - "
    h = h + ProjectName + VerStatusText
  END IF
  ' (Versions: what the experiment is, with Git's word for it)
  IF VerStatusText <> "" THEN
    IF h <> "" THEN h = h + " - "
    h = h + TR$("{experiment} is the experiment you're on (Git calls it a branch)", "experiment", Repo.Experiment)
  END IF
  Status.Hint = h
  Status.ShowHint = (h <> "")
  Status.AccessibleDescription = h
END SUB

' (POLISH-2) The room the state can have, in pixels of text: the bar's
' width but the file's facts (theirs kept) and a little of the project's
' name — at least the 230 pixels it always had.
FUNCTION StateRoom AS INTEGER
  Result = Studio.ClientWidth - 110 - 70 - 60 - 160 - 40
  IF Result < 230 THEN Result = 230
  Result = Result - 14
END FUNCTION

' Text as it fits in Room pixels of the status bar's font: whole, or with
' its middle taken out for "…" — its start and its end (a line number, a
' file's extension) kept.
FUNCTION FitMiddle(Text AS STRING, Room AS INTEGER) AS STRING
  DIM k AS INTEGER, n AS INTEGER, s AS STRING
  StatusMeasure.Font.Name = StatusFont.Name: StatusMeasure.Font.Size = StatusFont.Size
  IF StatusMeasure.TextWidth(Text) <= Room OR LEN(Text) < 6 THEN
    Result = Text
    EXIT FUNCTION
  END IF
  n = LEN(Text)
  FOR k = n - 1 TO 4 STEP -1
    s = LEFT$(Text, k \ 2) + "…" + RIGHT$(Text, k - k \ 2)
    IF StatusMeasure.TextWidth(s) <= Room THEN EXIT FOR
  NEXT
  Result = s
END FUNCTION

' (Text: in English, shown in Studio's language — LayOutStatus — so a
' change of language reaches it; a text made with values is TR$'s already)
SUB SetState(State AS STRING, Text AS STRING)
  RunState = State
  StateText = Text
  ApplyState
  UpdateCommands
END SUB

SUB SetUpStatus
  Status.AddPanels "", "", TR$("Ln {line}, Col {col}", "line", 1, "col", 1), "UTF-8", "CRLF", "RapidR BASIC"
  LayOutStatus
END SUB

' (the state at the left, the file's facts at the right; the project between.
' POLISH-2: the state is as wide as its words need — 230 pixels at least —,
' taking from the project's name, never from the facts; only words longer
' than the whole room are shortened, in their middle (a file's line number
' at the end stays), the whole of them the bar's tooltip)
SUB LayOutStatus
  DIM w AS INTEGER, w0 AS INTEGER, wp AS INTEGER
  StatusMeasure.Font.Name = StatusFont.Name: StatusMeasure.Font.Size = StatusFont.Size
  w0 = StatusMeasure.TextWidth(TR$(StateText)) + 14
  IF w0 < 230 THEN w0 = 230
  IF w0 > StateRoom + 14 THEN w0 = StateRoom + 14
  ' (the caret's place: 110 pixels hold the English words; a longer language
  ' with a line in the hundreds takes what its words need)
  wp = StatusMeasure.TextWidth(TR$("Ln {line}, Col {col}", "line", 999, "col", 99)) + 14
  IF wp < 110 THEN wp = 110
  w = Studio.ClientWidth - w0 - wp - 70 - 60 - 160 - McpStatusWidth
  IF w < 40 THEN w = 40
  ' (AI apps' panel, once they've been on: mcp.inc)
  IF McpStatusWidth > 0 THEN
    Status.Panel(5).Width = 160
    Status.Panel(6).Width = McpStatusWidth
  END IF
  Status.Panel(0).Width = w0
  Status.Panel(0).Caption = FitMiddle(TR$(StateText), w0 - 14)
  Status.Panel(1).Width = w
  Status.Panel(2).Width = wp
  Status.Panel(3).Width = 70
  Status.Panel(4).Width = 60
  ' (a layout's note there stays its four seconds: debug.inc's LayoutNote)
  IF LayoutNoteTimer.Enabled = 0 THEN
    ' (Versions: the experiment and the changes not saved as a version
    ' after the name — versions.inc)
    IF ProjectOpen THEN Status.Panel(1).Caption = FitMiddle(ProjectName + VerStatusText, w - 12 - LockChipRoom) ELSE Status.Panel(1).Caption = ""
  END IF
  ' (School Lock's chip: at the right end of the project's panel —
  ' schoollock.inc)
  LockChipPlace w0 + w - 6
  ShowStatusHint
END SUB

SUB UpdateTitle
  IF ProjectOpen THEN Studio.Caption = ProjectName + " - RapidR Studio" ELSE Studio.Caption = "RapidR Studio"
  LayOutStatus
END SUB

' (Text's width in Studio's UI font, its & left out: a layout that fits
' its words in the language Studio speaks)
FUNCTION UITextWidth(Text AS STRING) AS INTEGER
  StatusMeasure.Font.Name = UIFont.Name: StatusMeasure.Font.Size = UIFont.Size
  Result = StatusMeasure.TextWidth(REPLACESUBSTR$(Text, "&", ""))
END FUNCTION

' ---- Chinese on a computer with no font for it ---------------------------------
' RapidR draws Chinese with the computer's own fonts and bundles none (their
' licences allow it on the web only: docs/legal/i18n-zh-font.md). A computer
' with no Chinese font shows empty boxes: Studio says so, in the language and
' in English, and says how to add the system's own Chinese support (a Linux:
' the free package; Windows XP: its East Asian files, from the Windows CD;
' newer Windows: the Chinese language, whose fonts come with it). It installs
' nothing and names no other source of fonts. The test, on any desktop: the
' canvas draws a character no font has as a "?", so a Chinese character drawn
' the very same as "?" has no font. (RAPIDR_TEST_NO_CJK: the note, whatever
' the fonts, for the tests that look at it.)
DIM ZhProbeA AS RBitmap
DIM ZhProbeB AS RBitmap

FUNCTION ChineseFontMissing() AS INTEGER
  DIM x AS INTEGER, y AS INTEGER
  Result = 0
  IF ENVIRON$("RAPIDR_TEST_NO_CJK") <> "" THEN
    Result = 1
    EXIT FUNCTION
  END IF
  ZhProbeA.Width = 20: ZhProbeA.Height = 20
  ZhProbeB.Width = 20: ZhProbeB.Height = 20
  ZhProbeA.FillRect 0, 0, 20, 20, 16777215
  ZhProbeB.FillRect 0, 0, 20, 20, 16777215
  ZhProbeA.TextOut 1, 1, "你", 0, 16777215
  ZhProbeB.TextOut 1, 1, "?", 0, 16777215
  FOR y = 0 TO 19
    FOR x = 0 TO 19
      IF ZhProbeA.Pixel(x, y) <> ZhProbeB.Pixel(x, y) THEN EXIT FUNCTION
    NEXT
  NEXT
  Result = 1
END FUNCTION

' (Studio's Output, once at the start: only a desktop Studio in Chinese whose
' fonts have none for it — the web has its own font)
SUB ChineseFontNote
  DIM en AS STRING
  IF LEFT$(Application.Language, 2) <> "zh" THEN EXIT SUB
  IF ENVIRON$("RAPIDR_TEST_NO_CJK") = "" THEN
    IF Proj.FileManager = "" THEN EXIT SUB
  END IF
  IF ChineseFontMissing() = 0 THEN EXIT SUB
  SELECT CASE Application.OS
    CASE "Linux"
      en = "No font for Chinese was found on this computer, so Chinese text shows as empty boxes. Install the free package fonts-noto-cjk (for example: sudo apt install fonts-noto-cjk), then start Studio again."
      OutputLine en
      OutputLine TR$("No font for Chinese was found on this computer, so Chinese text shows as empty boxes. Install the free package fonts-noto-cjk (for example: sudo apt install fonts-noto-cjk), then start Studio again.")
    CASE "Windows"
      en = "No font for Chinese was found on this computer, so Chinese text shows as empty boxes. On Windows XP: Control Panel > Regional and Language Options > Languages > Install files for East Asian languages (it needs the Windows CD). On newer Windows: Settings > Time & Language > Language > add Chinese (Simplified); its fonts come with it. Then start Studio again."
      OutputLine en
      OutputLine TR$("No font for Chinese was found on this computer, so Chinese text shows as empty boxes. On Windows XP: Control Panel > Regional and Language Options > Languages > Install files for East Asian languages (it needs the Windows CD). On newer Windows: Settings > Time & Language > Language > add Chinese (Simplified); its fonts come with it. Then start Studio again.")
    CASE ELSE
      en = "No font for Chinese was found on this computer, so Chinese text shows as empty boxes. Add Chinese (Simplified) in the system's language settings, then start Studio again."
      OutputLine en
      OutputLine TR$("No font for Chinese was found on this computer, so Chinese text shows as empty boxes. Add Chinese (Simplified) in the system's language settings, then start Studio again.")
  END SELECT
END SUB

' ---- line breaks in Chinese, Japanese and Korean -------------------------------
' Those scripts have no spaces between words, so a line may end between two of
' their characters: rapidr_value::autosize::can_break_between, which wraps the
' labels, with the same rules. The estimators of a wrapped text's height
' (AISetLinesHigh, VerWrap; AITextHigh and AIStepsHigh in ai.inc are to
' follow) take a word apart there with BreakUnits, as the label breaks it.

' (a character of those scripts: ideographs, kana, hangul, their marks and
' full-width forms)
FUNCTION IsCjkChar(C AS STRING) AS INTEGER
  DIM u AS LONG
  Result = 0
  IF C = "" THEN EXIT FUNCTION
  u = ASC(C)
  IF u < &H2E80 THEN EXIT FUNCTION
  IF u <= &H303F THEN Result = 1: EXIT FUNCTION
  IF u >= &H3040 AND u <= &H312F THEN Result = 1: EXIT FUNCTION
  IF u >= &H3400 AND u <= &H4DBF THEN Result = 1: EXIT FUNCTION
  IF u >= &H4E00 AND u <= &H9FFF THEN Result = 1: EXIT FUNCTION
  IF u >= &HAC00 AND u <= &HD7AF THEN Result = 1: EXIT FUNCTION
  IF u >= &HF900 AND u <= &HFAFF THEN Result = 1: EXIT FUNCTION
  IF u >= &HFE30 AND u <= &HFE4F THEN Result = 1: EXIT FUNCTION
  IF u >= &HFF00 AND u <= &HFFEF THEN Result = 1
END FUNCTION

' (whether a line may end between A and the B after it: one of them is of
' those scripts, no line starts with a closing mark or ends with an opening
' one)
FUNCTION CanBreakBetween(A AS STRING, B AS STRING) AS INTEGER
  Result = 0
  IF A = " " OR B = " " THEN EXIT FUNCTION
  IF IsCjkChar(A) = 0 AND IsCjkChar(B) = 0 THEN EXIT FUNCTION
  IF INSTR("，。、；：？！）】》〉」』”’…—·％‰．］｝〕,.;:!?)]}%", B) > 0 THEN EXIT FUNCTION
  IF INSTR("（【《〈「『“‘［｛〔([{", A) > 0 THEN EXIT FUNCTION
  Result = 1
END FUNCTION

' (Wd with CHR$(1) at each place a line may end inside it: a word without
' Chinese, Japanese or Korean comes back as it is)
FUNCTION BreakUnits(Wd AS STRING) AS STRING
  DIM j AS INTEGER, has AS INTEGER
  Result = Wd
  FOR j = 1 TO LEN(Wd)
    IF ASC(MID$(Wd, j, 1)) >= &H2E80 THEN
      has = 1
      EXIT FOR
    END IF
  NEXT
  IF has = 0 THEN EXIT FUNCTION
  Result = ""
  FOR j = 1 TO LEN(Wd)
    IF j > 1 THEN
      IF CanBreakBetween(MID$(Wd, j - 1, 1), MID$(Wd, j, 1)) = 1 THEN Result = Result + CHR$(1)
    END IF
    Result = Result + MID$(Wd, j, 1)
  NEXT
END FUNCTION

' Tools > Settings > General > Language: the language Studio speaks — ""
' the system's, "en", "es" … (StudioLanguageChanged follows, at once).
SUB SetLanguage(Tag AS STRING)
  LanguageChoice = Tag
  IF Tag = "" THEN Application.Language = "" ELSE Application.Language = Tag
END SUB

' (Application.OnLanguageChange) Studio's language changed: captions, hints
' and accessible names that are a catalog's text whole were translated
' again before this (the runtime's); what Studio made itself — the tool
' bar's tips, the palette, lists' items, the status bar, the page texts
' made with values — is made again, and what is laid out by its text's
' width laid out again (a longer language: nothing cut off).
SUB StudioLanguageChanged
  DIM i AS INTEGER
  ' (the Outline's and Problems' texts made again in the new language)
  OutlineShownDoc = -1
  ProblemsShownPath = ""
  FOR i = 0 TO CmdCount - 1
    IF Cmd(i).Tool >= 0 THEN
      IF CmdKeys(i) <> "" THEN ToolBar.ButtonHint Cmd(i).Id, PlainTitle(Cmd(i).Title) + " (" + CmdKeys(i) + ")" ELSE ToolBar.ButtonHint Cmd(i).Id, PlainTitle(Cmd(i).Title)
    END IF
  NEXT
  FillPalette
  FillRecent
  ' (each part's own lists, titles and page texts: made again)
  PanesLanguageChanged
  PanelsLanguageChanged
  EditorLanguageChanged
  WorkspaceLanguageChanged
  MarkdownLanguageChanged
  SearchLanguageChanged
  DebugLanguageChanged
  AiLanguageChanged
  ' (AI Apps' Settings page before Settings is laid out again)
  McpLanguageChanged
  ' (the lists Settings fills once: filled again, the choice kept)
  SettingsLanguageChanged
  ' (School Lock's card, chip, strips, the commands' reasons)
  SchoolLockLanguageChanged
  StudioThemeChanged
  UpdateTitle
END SUB

' View > Theme: the look Studio is drawn in (StudioThemeChanged follows,
' as for the system's own changes).
SUB SetTheme(Name AS STRING)
  ThemeChoice = Name
  Application.Theme = Name
END SUB

' (Application.OnThemeChange) The look changed: View > Theme, or the system
' switched between light, dark and high contrast while Studio follows it
' (RapidR's look, View > Theme > RapidR). Every colour, icon, font and size
' Studio took from the theme is taken again — the start page, the panes,
' the status bar, the debugger's, and the dialogs' whether they are open or
' not — so nothing keeps the look it had.
SUB StudioThemeChanged
  ' (the outline's icons drawn again in the new look)
  OutlineShownDoc = -1
  QuickShownKey = ""
  ProblemsShownPath = ""
  ApplyChrome
  ' (the cards' titles are AutoSize: their fonts set again shrank them to
  ' their text — laid out again, as at the start)
  LayOutWelcome
  FillProjectTree
  AnalyzeActive
  UpdateCommands
  LayOutAppDialog
  LayOutSettingsDialog
  LayOutPrompt PromptMany
  PickTemplate TplChosen
END SUB

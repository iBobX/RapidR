' crash.inc: RapidR's crash and hang reports in Studio (rapidr-crash;
' RPROJECT's Crash… methods). Reports are written on this computer only;
' nothing is ever sent by RapidR.
'
' - At the start, after a Studio that ended unexpectedly (a crash in
'   RapidR's own code, a stack overflow, a hang it was closed in): a calm
'   strip under the tool bar — "RapidR Studio closed unexpectedly last
'   time." — with View Report (opens it in a tab), Copy Report, Save
'   Report..., Report on GitHub... (a question first: a GitHub account,
'   13+, public; then the browser on a prefilled new-issue page — the
'   person reads it and presses Submit, or not) and Dismiss. The strip
'   never takes the keyboard; a screen reader hears it. Not shown while
'   School Lock is on (CrashSchoolLock, the School Lock lane's hook).
' - A program run that RapidR stopped (a crash in RapidR's own code while
'   it ran): the Output says "Your program stopped: <what>, at main.rr line
'   120, in SUB Tick (OnTimer of Clock)." with links to the line and to the
'   report.
' - The hang watchdog: started with its question's words in Studio's
'   language (again when the language or look changes).
' - What a report shows of Studio: its messages (OutputLine), its last
'   command (RunCommand), the AI apps' calls by name (mcp.inc).
' - The tests' steps: crash:panic, crash:hang, crash:check, crash:view,
'   crash:copy, crash:save, crash:github=yes|no, crash:dismiss,
'   crash:report.

' (SetUpCrash, ApplyCrashChrome, CrashSchoolLock, CrashRunStarts,
' CrashProgramEnded, CrashLink, CrashStep: decl.inc)
DECLARE SUB CrashOfferTick
DECLARE SUB CrashCheck
DECLARE SUB CrashShowBar
DECLARE SUB CrashHideBar
DECLARE SUB CrashLayOut
DECLARE SUB CrashPick(Sender AS RButton)
DECLARE SUB CrashView(Name AS STRING)
DECLARE SUB CrashCopy(Name AS STRING)
DECLARE SUB CrashSaveAs(Name AS STRING)
DECLARE SUB CrashGitHub(Name AS STRING)
DECLARE SUB CrashWatchStart
DECLARE FUNCTION CrashButtonW(B AS RButton) AS INTEGER

' (the report the strip offers)
DIM CrashName AS STRING
' (School Lock is on: no strip, no GitHub — CrashSchoolLock)
DIM CrashLocked AS INTEGER
' (a run's start, for the report it may write: CrashForProgram)
DIM CrashRunMark AS STRING
' (the last program run's report, "" none)
DIM CrashProgramReport AS STRING
' (a test step answers Report on GitHub's question: 1 yes, 2 no)
DIM CrashAnswer AS INTEGER
' (what Report on GitHub opened last, and what Copy copied: crash:report)
DIM CrashLastURL AS STRING
DIM CrashCopied AS INTEGER
DIM CrashBmp AS RBitmap

CREATE CrashOfferTimer AS RTimer
  Interval = 600
  Enabled = 0
  OnTimer = CrashOfferTick
END CREATE

' The strip under the tool bar (a question's strip: docs/design-guidelines.md).
CREATE CrashBar AS RPanel
  BevelOuter = 0
  Visible = 0
  Height = 84
  CREATE CrashBarMark AS RPanel
    Left = 0: Top = 0: Width = 4: Height = 84
    BevelOuter = 0
    AccessibleHidden = 1
  END CREATE
  CREATE CrashBarIcon AS RImage
    Left = 16: Top = 11: Width = 16: Height = 16
    AccessibleHidden = 1
  END CREATE
  CREATE CrashBarText AS RLabel
    Left = 40: Top = 10: Height = 18
    AutoSize = 0
    AccessibleHidden = 1
  END CREATE
  CREATE CrashBarDetail AS RLabel
    Left = 40: Top = 30: Height = 18
    AutoSize = 0: WordWrap = 1
    AccessibleHidden = 1
  END CREATE
  CREATE CrashViewB AS RButton
    Caption = TR$("&View Report")
    OnClick = CrashPick
    Tag = 1
  END CREATE
  CREATE CrashCopyB AS RButton
    Caption = TR$("Cop&y Report")
    OnClick = CrashPick
    Tag = 2
  END CREATE
  CREATE CrashSaveB AS RButton
    Caption = TR$("Sa&ve Report...")
    OnClick = CrashPick
    Tag = 3
  END CREATE
  CREATE CrashGitB AS RButton
    Caption = TR$("Report on &GitHub...")
    OnClick = CrashPick
    Tag = 4
  END CREATE
  CREATE CrashDismissB AS RButton
    Caption = TR$("&Dismiss")
    OnClick = CrashPick
    Tag = 5
  END CREATE
END CREATE

SUB SetUpCrash
  CrashBar.Parent = Studio
  CrashBar.Align = 1
  CrashBar.Top = ToolBar.Top + ToolBar.Height
  CrashBar.OnResize = CrashLayOut
  CrashWatchStart
  ' (once Studio's window is up: a Studio that ended unexpectedly last time)
  CrashOfferTimer.Enabled = 1
END SUB

' The look and the words again (ApplyChrome: the theme, the language).
SUB ApplyCrashChrome
  CrashBar.Color = TC("card")
  CrashBarMark.Color = TC("status.warning.border")
  CrashBarText.Font = UIFont
  CrashBarText.Font.Bold = 1
  CrashBarText.Font.Color = TC("page.text")
  CrashBarText.Transparent = 1
  CrashBarDetail.Font = UIFont
  CrashBarDetail.Font.Color = TC("page.dim")
  CrashBarDetail.Transparent = 1
  CrashViewB.Font = UIFont: CrashViewB.Caption = TR$("&View Report")
  CrashViewB.AccessibleDescription = TR$("Opens the report in a tab")
  CrashCopyB.Font = UIFont: CrashCopyB.Caption = TR$("Cop&y Report")
  CrashCopyB.AccessibleDescription = TR$("Copies the report's text")
  CrashSaveB.Font = UIFont: CrashSaveB.Caption = TR$("Sa&ve Report...")
  CrashSaveB.AccessibleDescription = TR$("Saves a copy of the report where you choose")
  CrashGitB.Font = UIFont: CrashGitB.Caption = TR$("Report on &GitHub...")
  CrashGitB.AccessibleDescription = TR$("Opens a new issue on GitHub in your browser, filled in with the report. Nothing is sent until you send it there yourself.")
  CrashDismissB.Font = UIFont: CrashDismissB.Caption = TR$("&Dismiss")
  CrashDismissB.AccessibleDescription = TR$("Closes this note. The report stays on this computer for 30 days.")
  CrashBmp.LoadIcon "warning", 16
  CrashBarIcon.BMPHandle = CrashBmp
  IF CrashBar.Visible THEN CrashShowBar
  CrashWatchStart
END SUB

' (the watchdog's question in Studio's language and look: a window of its own)
SUB CrashWatchStart
  DIM ok AS INTEGER
  ok = Proj.CrashWatch(5, TR$("RapidR Studio isn't responding"), TR$("Its window hasn't answered for a few seconds. Wait gives it more time. Save Copies and Quit closes it: the copies of your unsaved work it keeps as you type (every 20 seconds) are offered when it starts again. A report was written on this computer."), TR$("&Wait"), TR$("&Save Copies and Quit"), ThemeName, Application.Language)
END SUB

' (School Lock's hook: on, no strip and no Report on GitHub)
SUB CrashSchoolLock(Locked AS INTEGER)
  CrashLocked = Flag(Locked <> 0)
  IF CrashLocked THEN CrashHideBar
END SUB

SUB CrashOfferTick
  CrashOfferTimer.Enabled = 0
  CrashCheck
END SUB

' A Studio that ended unexpectedly: its report offered in the strip.
SUB CrashCheck
  DIM n AS STRING
  IF CrashLocked THEN EXIT SUB
  n = Proj.CrashPending
  IF n = "" THEN EXIT SUB
  CrashName = n
  CrashShowBar
  Application.Announce CrashBar.AccessibleName, 0
END SUB

SUB CrashShowBar
  DIM t AS STRING, d AS STRING
  IF CrashName = "" THEN EXIT SUB
  IF Proj.CrashField(CrashName, "Kind") = "hang" THEN
    t = TR$("RapidR Studio stopped responding last time and was closed.")
  ELSE
    t = TR$("RapidR Studio closed unexpectedly last time.")
  END IF
  d = TR$("A report of what happened stays on this computer; nothing was sent. View it, copy it, save it, or report the problem on GitHub to help fix it.")
  CrashBarText.Caption = t
  CrashBarDetail.Caption = d
  CrashBar.AccessibleName = t + " " + d
  CrashBar.Hint = Proj.CrashPath(CrashName)
  CrashGitB.Visible = Flag(CrashLocked = 0)
  CrashLayOut
  CrashBar.Visible = 1
END SUB

SUB CrashHideBar
  CrashBar.Visible = 0
END SUB

' (a compact button as wide as its words: 12 each side, 72 at least)
FUNCTION CrashButtonW(B AS RButton) AS INTEGER
  Result = UITextWidth(B.Caption) + 24
  IF Result < 72 THEN Result = 72
END FUNCTION

' The strip: the icon and the bold sentence, the second sentence under it,
' the buttons under them — compact, 8 apart, from where the words start.
SUB CrashLayOut
  DIM w AS INTEGER, ch AS INTEGER, x AS INTEGER, y AS INTEGER, h AS INTEGER, dh AS INTEGER
  ch = UiCompactH
  w = CrashBar.ClientWidth
  IF w < 200 THEN w = Studio.ClientWidth
  IF w < 200 THEN w = 800
  ' (the second sentence wraps in a narrow window or a longer language)
  AISetMeasure.Font.Name = UIFont.Name: AISetMeasure.Font.Size = UIFont.Size
  dh = AISetLinesHigh(CrashBarDetail.Caption, w - 52)
  IF dh < 18 THEN dh = 18
  CrashBarIcon.Left = 16: CrashBarIcon.Top = 11
  CrashBarText.Left = 40: CrashBarText.Top = 10: CrashBarText.Width = w - 52: CrashBarText.Height = 18
  CrashBarDetail.Left = 40: CrashBarDetail.Top = 30: CrashBarDetail.Width = w - 52: CrashBarDetail.Height = dh
  y = 30 + dh + 8
  x = 40
  CrashViewB.Left = x: CrashViewB.Top = y: CrashViewB.Width = CrashButtonW(CrashViewB): CrashViewB.Height = ch
  x = x + CrashViewB.Width + 8
  CrashCopyB.Left = x: CrashCopyB.Top = y: CrashCopyB.Width = CrashButtonW(CrashCopyB): CrashCopyB.Height = ch
  x = x + CrashCopyB.Width + 8
  CrashSaveB.Left = x: CrashSaveB.Top = y: CrashSaveB.Width = CrashButtonW(CrashSaveB): CrashSaveB.Height = ch
  x = x + CrashSaveB.Width + 8
  IF CrashGitB.Visible THEN
    CrashGitB.Left = x: CrashGitB.Top = y: CrashGitB.Width = CrashButtonW(CrashGitB): CrashGitB.Height = ch
    x = x + CrashGitB.Width + 8
  END IF
  CrashDismissB.Left = x: CrashDismissB.Top = y: CrashDismissB.Width = CrashButtonW(CrashDismissB): CrashDismissB.Height = ch
  h = y + ch + 12
  IF CrashBar.Height <> h THEN CrashBar.Height = h
  CrashBarMark.Left = 0: CrashBarMark.Top = 0: CrashBarMark.Height = h
END SUB

SUB CrashPick(Sender AS RButton)
  IF CrashName = "" THEN EXIT SUB
  ' (any answer: it isn't offered again at the next start)
  Proj.CrashSeen CrashName
  SELECT CASE Sender.Tag
    CASE 1: CrashView CrashName
    CASE 2: CrashCopy CrashName
    CASE 3: CrashSaveAs CrashName
    CASE 4: CrashGitHub CrashName
    CASE 5: CrashHideBar
  END SELECT
END SUB

' (in a tab: the report's file)
SUB CrashView(Name AS STRING)
  DIM p AS STRING
  p = Proj.CrashPath(Name)
  IF p = "" THEN
    OutputLine TR$("The report isn't there any more.")
    EXIT SUB
  END IF
  OpenDocument p
END SUB

SUB CrashCopy(Name AS STRING)
  Clipboard.Text = Proj.CrashText(Name)
  CrashCopied = LEN(Clipboard.Text)
  LayoutNote TR$("The report was copied.")
END SUB

SUB CrashSaveAs(Name AS STRING)
  SaveDlg.Caption = TR$("Save Report")
  SaveDlg.FileName = Name
  SaveDlg.Filter = TR$("Text files") + " (*.txt)|*.txt|" + TR$("All files") + "|*.*"
  IF SaveDlg.Execute = 0 THEN EXIT SUB
  IF Proj.CrashSave(Name, SaveDlg.FileName) THEN
    OutputLine TR$("Saved the report as {file}.", "file", SaveDlg.FileName)
    LayoutNote TR$("Saved the report as {file}.", "file", FileTitle(SaveDlg.FileName))
  ELSE
    OutputLine TR$("Couldn't save the report: {error}", "error", Proj.Error)
  END IF
END SUB

' Report on GitHub: asked first (what opens, what it shows, who can, that
' it's public); then the browser on RapidR's new-issue page, filled in —
' the person reads it there and decides. A report too long for the
' address is copied first, to paste.
SUB CrashGitHub(Name AS STRING)
  DIM yes AS INTEGER, url AS STRING
  IF CrashLocked THEN EXIT SUB
  url = Proj.CrashIssueURL(Name)
  IF url = "" THEN EXIT SUB
  IF CrashAnswer THEN
    ' (a test's step)
    yes = Flag(CrashAnswer = 1)
  ELSE
    ConsentBegin TR$("Report this on GitHub?"), TR$("Your browser opens RapidR's page for a new issue on GitHub, filled in with this report. Nothing is sent until you send it there yourself.")
    ConsentRow "visible", TR$("It shows"), TR$("RapidR's version, your system, where it stopped, its last messages and the names of the last AI calls. Your home folder appears as ~; no file contents, keys or passwords.")
    ConsentRow "info", TR$("You need"), TR$("A GitHub account. GitHub's terms ask that you be 13 or older; a younger student can ask a teacher or a parent to report it.")
    ConsentRow "warning", TR$("Keep in mind"), TR$("Issues on GitHub are public: anyone can read them. Read the report first, and take out anything you don't want to share.")
    yes = ConsentAsk("", "", TR$("&Open GitHub"), TR$("Cancel"))
  END IF
  IF yes = 0 THEN EXIT SUB
  IF Proj.CrashIssueWhole(Name) = 0 THEN
    Clipboard.Text = Proj.CrashText(Name)
    CrashCopied = LEN(Clipboard.Text)
    OutputLine TR$("The page has a shorter report; the whole one was copied: paste it into the issue's text too.")
  END IF
  CrashLastURL = url
  OpenWebLink url
END SUB

' ---- a program run that RapidR stopped -------------------------------------------------------

' (Run: the time it started, for the report it may write)
SUB CrashRunStarts
  CrashRunMark = Proj.CrashMark
  CrashProgramReport = ""
END SUB

' (the run ended: with a report of RapidR's, the Output says so plainly)
SUB CrashProgramEnded(Code AS INTEGER)
  DIM n AS STRING, what AS STRING, where AS STRING, f AS STRING, ln AS STRING, r AS STRING, ev AS STRING, comp AS STRING
  IF Code = 0 OR CrashRunMark = "" THEN EXIT SUB
  n = Proj.CrashForProgram(CrashRunMark, ProjectMain)
  CrashRunMark = ""
  IF n = "" THEN EXIT SUB
  CrashProgramReport = n
  IF Proj.CrashField(n, "Kind") = "stack-overflow" THEN
    what = TR$("the stack overflowed (calls went too deep)")
  ELSE
    what = Proj.CrashField(n, "Message")
  END IF
  ' (the place in Studio's language: "main.rr line 120" a link to the
  ' line, the SUB, the event and its component)
  f = Proj.CrashField(n, "WhereFile")
  ln = Proj.CrashField(n, "WhereLine")
  r = Proj.CrashField(n, "WhereRoutine")
  ev = Proj.CrashField(n, "WhereEvent")
  comp = Proj.CrashField(n, "WhereComponent")
  where = ""
  IF VAL(ln) > 0 THEN
    IF f <> "" THEN
      where = Hyperlink(TR$("{file} line {n}", "file", f, "n", ln), "crash-at:" + f + ":" + ln)
    ELSE
      where = TR$("line {n}", "n", ln)
    END IF
    IF r <> "" THEN where = TR$("{place}, in {routine}", "place", where, "routine", r) ELSE where = TR$("{place}, in the main program", "place", where)
  END IF
  IF ev <> "" THEN
    IF comp <> "" THEN ev = TR$("{event} of {component}", "event", ev, "component", comp)
    IF where <> "" THEN where = where + " (" + ev + ")" ELSE where = ev
  END IF
  IF where <> "" THEN
    OutputAfterProgram TR$("Your program stopped: {what}, at {where}.", "what", what, "where", where)
  ELSE
    OutputAfterProgram TR$("Your program stopped: {what}.", "what", what)
  END IF
  OutputLine TR$("RapidR stopped it: a problem in RapidR, not in your code. A report stays on this computer:") + " " + Hyperlink(TR$("View Report"), "crash-report:" + n)
  SetState "error", TR$("Your program stopped: a problem in RapidR")
  Application.Announce TR$("Your program stopped: {what}.", "what", what), 1
END SUB

' (the Output's crash-at: and crash-report: links — OutputLink)
SUB CrashLink(Target AS STRING)
  DIM rest AS STRING, k AS INTEGER
  IF LEFT$(Target, 13) = "crash-report:" THEN
    CrashView MID$(Target, 14)
  ELSEIF LEFT$(Target, 9) = "crash-at:" THEN
    rest = MID$(Target, 10)
    k = RINSTR(rest, ":")
    IF k > 1 THEN OutputLink LEFT$(rest, k - 1), VAL(MID$(rest, k + 1))
  END IF
END SUB

' ---- the tests' steps -----------------------------------------------------------------------

FUNCTION CrashStep(Stp AS STRING) AS INTEGER
  DIM ok AS INTEGER, s AS STRING
  IF LEFT$(Stp, 6) <> "crash:" THEN EXIT FUNCTION
  Result = 1
  s = MID$(Stp, 7)
  SELECT CASE s
    CASE "panic", "hang", "overflow", "abort"
      ok = Proj.CrashTest(s)
    CASE "check"
      CrashCheck
    CASE "view"
      CrashPick CrashViewB
    CASE "copy"
      CrashPick CrashCopyB
    CASE "save"
      CrashPick CrashSaveB
    CASE "github=yes", "github=no"
      CrashAnswer = IIF(s = "github=yes", 1, 2)
      CrashPick CrashGitB
      CrashAnswer = 0
    CASE "github"
      CrashPick CrashGitB
    CASE "dismiss"
      CrashPick CrashDismissB
    CASE "lock"
      CrashSchoolLock 1
    CASE "report"
      ' (the newest report's outcome: a hang that ended)
      OutputLine "[crash] bar=" + Num(CrashBar.Visible) + " report=" + CrashName + " kind=" + Proj.CrashField(CrashName, "Kind") + " pending=" + Proj.CrashPending + " copied=" + Num(CrashCopied) + " program=" + CrashProgramReport + " list=" + REPLACESUBSTR$(Proj.CrashList, CHR$(10), " ") + "outcome=" + Proj.CrashField(FIELD$(Proj.CrashList, CHR$(10), 1), "Outcome") + " url=" + CrashLastURL
  END SELECT
END FUNCTION

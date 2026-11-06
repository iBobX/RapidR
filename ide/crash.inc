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
DECLARE SUB CrashChoose(Index AS INTEGER, Caption AS STRING)
DECLARE SUB CrashAct(Tag AS INTEGER)
DECLARE SUB CrashView(Name AS STRING)
DECLARE SUB CrashCopy(Name AS STRING)
DECLARE SUB CrashSaveAs(Name AS STRING)
DECLARE SUB CrashGitHub(Name AS STRING)
DECLARE SUB CrashWatchStart

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
' (the strip's buttons, in order: "1,2,3,4,5" — View, Copy, Save, GitHub,
' Dismiss —, without 4 under School Lock)
DIM CrashTags AS STRING

CREATE CrashOfferTimer AS RTimer
  Interval = 600
  Enabled = 0
  OnTimer = CrashOfferTick
END CREATE

' The question under the tool bar: one slim line across the window
' (RNotifyBar, docs/design-guidelines.md "Notifications"). The buttons
' are, in this order, View Report, Copy Report, Save Report..., Report on
' GitHub... (not while School Lock is on) and Dismiss; CrashTags says which
' one each is.
CREATE CrashBar AS RNotifyBar
  Align = 1
  OnChoose = CrashChoose
END CREATE

SUB SetUpCrash
  CrashBar.Parent = Studio
  CrashBar.Align = 1
  CrashBar.Top = ToolBar.Top + ToolBar.Height
  CrashWatchStart
  ' (once Studio's window is up: a Studio that ended unexpectedly last time)
  CrashOfferTimer.Enabled = 1
END SUB

' The look and the words again (ApplyChrome: the theme, the language).
SUB ApplyCrashChrome
  CrashBar.Font = UIFont
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
END SUB

SUB CrashShowBar
  DIM t AS STRING, b AS STRING
  IF CrashName = "" THEN EXIT SUB
  IF Proj.CrashField(CrashName, "Kind") = "hang" THEN
    t = TR$("RapidR Studio stopped responding last time and was closed.")
  ELSE
    t = TR$("RapidR Studio closed unexpectedly last time.")
  END IF
  t = t + " " + TR$("A report stays on this computer; nothing was sent.")
  b = TR$("&View Report") + "," + TR$("Cop&y Report") + "," + TR$("Sa&ve Report...") + ","
  CrashTags = "1,2,3,"
  IF CrashLocked = 0 THEN
    b = b + TR$("Report on &GitHub...") + ","
    CrashTags = CrashTags + "4,"
  END IF
  b = b + TR$("&Dismiss")
  CrashTags = CrashTags + "5"
  CrashBar.Hint = Proj.CrashPath(CrashName)
  ' (asked once, aloud: the strip itself says it to a screen reader)
  CrashBar.Ask t, b, "warning"
END SUB

SUB CrashHideBar
  CrashBar.Hide
END SUB

' The strip's button (its number from 1): which action it is, by CrashTags.
SUB CrashChoose(Index AS INTEGER, Caption AS STRING)
  CrashAct VAL(FIELD$(CrashTags, ",", Index))
END SUB

SUB CrashAct(Tag AS INTEGER)
  IF CrashName = "" THEN EXIT SUB
  ' (any answer: it isn't offered again at the next start)
  Proj.CrashSeen CrashName
  SELECT CASE Tag
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
      CrashAct 1
    CASE "copy"
      CrashAct 2
    CASE "save"
      CrashAct 3
    CASE "github=yes", "github=no"
      CrashAnswer = IIF(s = "github=yes", 1, 2)
      CrashAct 4
      CrashAnswer = 0
    CASE "github"
      CrashAct 4
    CASE "dismiss"
      CrashAct 5
    CASE "lock"
      CrashSchoolLock 1
    CASE "report"
      ' (the newest report's outcome: a hang that ended)
      OutputLine "[crash] bar=" + Num(CrashBar.Visible) + " report=" + CrashName + " kind=" + Proj.CrashField(CrashName, "Kind") + " pending=" + Proj.CrashPending + " copied=" + Num(CrashCopied) + " program=" + CrashProgramReport + " list=" + REPLACESUBSTR$(Proj.CrashList, CHR$(10), " ") + "outcome=" + Proj.CrashField(FIELD$(Proj.CrashList, CHR$(10), 1), "Outcome") + " url=" + CrashLastURL
  END SELECT
END FUNCTION

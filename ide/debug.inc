' debug.inc: running the program and debugging it (docs/ide-plan.md I6,
' docs/studio-wow.md RUN / DBG): Run (F5) and Run Without Debugging, Run in
' Browser, the breakpoints (F9 or a click in the gutter; conditions, hit
' counts, logpoints, switched off), where the program is paused (the line
' marked and shown, in whichever file), stepping, Run to Cursor, and the
' debugger's panes: Variables, Watch, Call Stack, Breakpoints, Immediate,
' and the values a resting mouse shows while paused.
'
' The program runs in RProgramSession (rapidr-studio's session.rs): its own
' process on the desktop, a sandboxed frame on the web. At a stop the
' session fetches the call stack, the selected frame's locals, the globals
' and the watches, then fires OnStopped; anything more (an object opened in
' a tree, an expression) comes back as OnVariables / OnEvaluate.
'
' The code editor shows the debugger's state with markers (RCODEEDITOR's
' AddMarker): "breakpoint", "breakpoint.conditional", "breakpoint.log",
' "breakpoint.disabled", "current" (where it's paused), "frame" (a caller's
' line picked in the Call Stack) and "exception" (a run-time error's line,
' its message at the line's end).

' ---- the breakpoints ------------------------------------------------------------------
' Every breakpoint of the project, open document or not: its file (as the
' project has it), line (from 1), condition, hit count (3, >= 3, % 3 …),
' log message (a logpoint: it prints and goes on) and whether it's on.
CONST MAXBPS = 200
DIM BpFile(MAXBPS) AS STRING
DIM BpLine(MAXBPS) AS INTEGER
DIM BpCond(MAXBPS) AS STRING
DIM BpHit(MAXBPS) AS STRING
DIM BpLog(MAXBPS) AS STRING
DIM BpOn(MAXBPS) AS INTEGER
' (the running program found no code at or after its line: it never stops)
DIM BpNever(MAXBPS) AS INTEGER
DIM BpCount AS INTEGER
' (the Breakpoints pane's selected one, -1: none; its edits being filled in)
DIM BpPicked AS INTEGER
DIM BpFilling AS INTEGER

' ---- the debugger's panes ---------------------------------------------------------------
DIM VarsTree AS QTREEVIEW
DIM WatchPanel AS QPANEL
DIM WatchStrip AS QPANEL
DIM WatchLabel AS QLABEL
DIM WatchEdit AS QEDIT
DIM WatchTree AS QTREEVIEW
DIM StackTree AS QTREEVIEW
DIM BpPanel AS QPANEL
DIM BpTree AS QTREEVIEW
DIM BpStrip AS QPANEL
DIM BpCondLabel AS QLABEL
DIM BpCondEdit AS QEDIT
DIM BpHitLabel AS QLABEL
DIM BpHitEdit AS QEDIT
DIM BpLogLabel AS QLABEL
DIM BpLogEdit AS QEDIT
DIM BpOnCheck AS QCHECKBOX

' The trees' items: what each shows (the variables' reference: its children,
' 0 none; its path, so what was open stays open from stop to stop).
CONST MAXVARITEMS = 1500
DIM VarItemRef(MAXVARITEMS) AS INTEGER
DIM VarItemPath(MAXVARITEMS) AS STRING
DIM WatchItemRef(MAXVARITEMS) AS INTEGER
DIM WatchItemPath(MAXVARITEMS) AS STRING
' (a tree being refilled: its expand events aren't the user's)
DIM TreeFilling AS INTEGER
' "|path|…": what the user opened in Variables and in Watch
DIM VarsOpen AS STRING
DIM WatchOpen AS STRING
' "|ref|…": what this stop has fetched (references live until the program goes on)
DIM RefsIn AS STRING
' (the stack's frames as the last stop fetched them: name, file, line)
DIM FrameName(64) AS STRING
DIM FrameFile(64) AS STRING
DIM FrameLine(64) AS INTEGER
DIM FrameCount AS INTEGER

' (what Studio asked the session to evaluate: the Immediate window's line,
' a data tip's expression and the editor it shows in)
DIM ImmEvalId AS INTEGER
DIM HoverEvalId AS INTEGER
DIM HoverDoc AS INTEGER
DIM HoverExpr AS STRING
' (the Immediate window's lines typed, for Up / Down)
DIM ImmHistory(50) AS STRING
DIM ImmCount AS INTEGER
DIM ImmAt AS INTEGER
' (where the program is paused, its file as Studio opened it)
DIM PausedFile AS STRING
DIM PausedLine AS INTEGER
DIM BreakOnErrors AS INTEGER
' (the budgets' measures, ms: F5 to the program's first form, a step to its
' line shown with its values; -1 not measured)
DIM PerfForm AS INTEGER
DIM PerfStep AS INTEGER
DIM StepStarted AS DOUBLE
DIM PerfReady AS QTIMER
PerfReady.Interval = 1
PerfReady.Enabled = 0
PerfReady.OnTimer = PerfReadyTick

SUB PerfReadyTick
  PerfReady.Enabled = 0
  PRINT "[perf] ready " + STR$(TIMER)
END SUB

' ---- small helpers ------------------------------------------------------------------------

FUNCTION Num(n AS INTEGER) AS STRING
  Result = LTRIM$(STR$(n))
END FUNCTION

' (a file the program names — a path, or a name as its source map keeps it —
' as Studio has it: an open document's, the project's, beside the main file)
FUNCTION DebugPath(InFile AS STRING) AS STRING
  DIM f AS STRING, d AS INTEGER, i AS INTEGER, t AS STRING
  f = REPLACESUBSTR$(InFile, "\", "/")
  Result = f
  IF f = "" THEN EXIT FUNCTION
  t = LCASE$(FileTitle(f))
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) THEN
      IF LCASE$(DocFile(d)) = LCASE$(f) THEN Result = DocFile(d): EXIT FUNCTION
    END IF
  NEXT
  IF FILEEXISTS(f) AND (LEFT$(f, 1) = "/" OR MID$(f, 2, 1) = ":") THEN EXIT FUNCTION
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) THEN
      IF LCASE$(FileTitle(DocFile(d))) = t THEN Result = DocFile(d): EXIT FUNCTION
    END IF
  NEXT
  IF ProjectOpen THEN
    FOR i = 0 TO Proj.FileCount - 1
      IF LCASE$(FileTitle(Proj.FullPath(i))) = t THEN Result = Proj.FullPath(i): EXIT FUNCTION
    NEXT
  END IF
  IF FILEEXISTS(FolderOf(ProjectMain) + f) THEN Result = FolderOf(ProjectMain) + f
END FUNCTION

' (a path's folder, with its last "/")
FUNCTION FolderOf(Path AS STRING) AS STRING
  DIM p AS STRING
  p = REPLACESUBSTR$(Path, "\", "/")
  IF RINSTR(p, "/") > 0 THEN Result = LEFT$(p, RINSTR(p, "/")) ELSE Result = ""
END FUNCTION

' (the code document d's pane)
' ---- breakpoints ----------------------------------------------------------------------------

FUNCTION FindBreakpoint(InFile AS STRING, LineNo AS INTEGER) AS INTEGER
  DIM i AS INTEGER
  Result = -1
  FOR i = 0 TO BpCount - 1
    IF BpLine(i) = LineNo AND LCASE$(BpFile(i)) = LCASE$(InFile) THEN Result = i: EXIT FUNCTION
  NEXT
END FUNCTION

' (its marker's kind: what the gutter draws)
FUNCTION BpKind(i AS INTEGER) AS STRING
  IF BpOn(i) = 0 OR BpNever(i) THEN
    Result = "breakpoint.disabled"
  ELSEIF BpLog(i) <> "" THEN
    Result = "breakpoint.log"
  ELSEIF BpCond(i) <> "" OR BpHit(i) <> "" THEN
    Result = "breakpoint.conditional"
  ELSE
    Result = "breakpoint"
  END IF
END FUNCTION

SUB UnmarkBreakpoint(i AS INTEGER)
  DIM d AS INTEGER
  d = DocOf(BpFile(i))
  IF d < 0 THEN EXIT SUB
  CodeDoc(d).RemoveMarker BpLine(i), "breakpoint"
  CodeDoc(d).RemoveMarker BpLine(i), "breakpoint.conditional"
  CodeDoc(d).RemoveMarker BpLine(i), "breakpoint.log"
  CodeDoc(d).RemoveMarker BpLine(i), "breakpoint.disabled"
END SUB

SUB MarkBreakpoint(i AS INTEGER)
  DIM d AS INTEGER
  d = DocOf(BpFile(i))
  IF d < 0 THEN EXIT SUB
  UnmarkBreakpoint i
  CodeDoc(d).AddMarker BpLine(i), BpKind(i)
END SUB

' (the session told: the program stops there — or prints, or not at all)
SUB SendBreakpoint(i AS INTEGER)
  IF BpOn(i) THEN
    Session.SetBreakpoint BpFile(i), BpLine(i), BpCond(i), BpHit(i), BpLog(i)
  ELSE
    Session.ClearBreakpoint BpFile(i), BpLine(i)
  END IF
END SUB

FUNCTION AddBreakpoint(InFile AS STRING, LineNo AS INTEGER) AS INTEGER
  DIM i AS INTEGER
  Result = FindBreakpoint(InFile, LineNo)
  IF Result >= 0 OR BpCount >= MAXBPS THEN EXIT FUNCTION
  i = BpCount
  BpCount = BpCount + 1
  BpFile(i) = InFile: BpLine(i) = LineNo
  BpCond(i) = "": BpHit(i) = "": BpLog(i) = "": BpOn(i) = 1: BpNever(i) = 0
  MarkBreakpoint i
  SendBreakpoint i
  Result = i
  DebugStateChanged
END FUNCTION

SUB RemoveBreakpoint(i AS INTEGER)
  DIM k AS INTEGER
  IF i < 0 OR i >= BpCount THEN EXIT SUB
  UnmarkBreakpoint i
  Session.ClearBreakpoint BpFile(i), BpLine(i)
  FOR k = i TO BpCount - 2
    BpFile(k) = BpFile(k + 1): BpLine(k) = BpLine(k + 1)
    BpCond(k) = BpCond(k + 1): BpHit(k) = BpHit(k + 1)
    BpLog(k) = BpLog(k + 1): BpOn(k) = BpOn(k + 1): BpNever(k) = BpNever(k + 1)
  NEXT
  BpCount = BpCount - 1
  IF BpPicked >= BpCount THEN BpPicked = BpCount - 1
  DebugStateChanged
END SUB

' F9 / a click in the gutter: a breakpoint on the line, or none any more.
SUB ToggleBreakpointAt(d AS INTEGER, LineNo AS INTEGER)
  DIM i AS INTEGER
  IF d < 0 OR LineNo < 1 THEN EXIT SUB
  SyncBreakpointLines d
  i = FindBreakpoint(DocFile(d), LineNo)
  IF i >= 0 THEN
    RemoveBreakpoint i
    SetState RunState, "Breakpoint removed: " + FileTitle(DocFile(d)) + ", line " + Num(LineNo)
  ELSE
    BpPicked = AddBreakpoint(DocFile(d), LineNo)
    SetState RunState, "Breakpoint at " + FileTitle(DocFile(d)) + ", line " + Num(LineNo)
  END IF
  FillBreakpoints
END SUB

' A breakpoint's rules changed (the Breakpoints pane): shown and sent again.
SUB ChangeBreakpoint(i AS INTEGER, Cond AS STRING, Hit AS STRING, LogMsg AS STRING, IsOn AS INTEGER)
  IF i < 0 OR i >= BpCount THEN EXIT SUB
  BpCond(i) = LTRIM$(RTRIM$(Cond))
  BpHit(i) = LTRIM$(RTRIM$(Hit))
  BpLog(i) = LogMsg
  BpOn(i) = Flag(IsOn)
  BpNever(i) = 0
  MarkBreakpoint i
  SendBreakpoint i
  FillBreakpoints
  DebugStateChanged
END SUB

' The running program placed a breakpoint elsewhere than its line (one on a
' comment, a blank line, a DIM): it moves to the line the program stops at,
' as Xcode and VS Code show it; with no code after it in its file, it never
' stops — drawn as a ring, and the Breakpoints pane says so.
SUB BreakpointPlaced(InFile AS STRING, LineNo AS INTEGER, NewLine AS INTEGER)
  DIM i AS INTEGER, j AS INTEGER
  i = FindBreakpoint(InFile, LineNo)
  IF i < 0 THEN i = FindBreakpoint(DebugPath(InFile), LineNo)
  IF i < 0 THEN EXIT SUB
  IF NewLine <= 0 THEN
    UnmarkBreakpoint i
    BpNever(i) = 1
    MarkBreakpoint i
  ELSE
    j = FindBreakpoint(BpFile(i), NewLine)
    IF j >= 0 THEN
      ' (one there already: the moved one goes)
      RemoveBreakpoint i
    ELSE
      UnmarkBreakpoint i
      Session.ClearBreakpoint BpFile(i), BpLine(i)
      BpLine(i) = NewLine
      MarkBreakpoint i
      SendBreakpoint i
      DebugStateChanged
    END IF
  END IF
  FillBreakpoints
END SUB

SUB ClearAllBreakpoints
  DO WHILE BpCount > 0
    RemoveBreakpoint BpCount - 1
  LOOP
  Session.ClearBreakpoints
  BpPicked = -1
  FillBreakpoints
  SetState RunState, "Every breakpoint removed"
END SUB

' The lines moved by editing follow the markers: each kind's breakpoints of
' file d, in order, take its markers' lines in order (the session told
' where they are now).
SUB SyncBreakpointLines(d AS INTEGER)
  DIM k AS INTEGER, kind AS STRING, lines AS STRING, n AS INTEGER, i AS INTEGER, j AS INTEGER, m AS INTEGER, nl AS INTEGER
  DIM idx(MAXBPS) AS INTEGER
  IF d < 0 THEN EXIT SUB
  FOR k = 0 TO 3
    SELECT CASE k
      CASE 0: kind = "breakpoint"
      CASE 1: kind = "breakpoint.conditional"
      CASE 2: kind = "breakpoint.log"
      CASE ELSE: kind = "breakpoint.disabled"
    END SELECT
    ' (this kind's breakpoints of the file, by line)
    n = 0
    FOR i = 0 TO BpCount - 1
      IF LCASE$(BpFile(i)) = LCASE$(DocFile(d)) AND BpKind(i) = kind THEN
        j = n
        DO WHILE j > 0
          IF BpLine(idx(j - 1)) <= BpLine(i) THEN EXIT DO
          idx(j) = idx(j - 1)
          j = j - 1
        LOOP
        idx(j) = i
        n = n + 1
      END IF
    NEXT
    lines = CodeDoc(d).GetMarkers(kind)
    IF lines <> "" THEN m = TALLY(lines, ",") + 1 ELSE m = 0
    IF m = n THEN
      FOR j = 0 TO n - 1
        nl = VAL(FIELD$(lines, ",", j + 1))
        IF nl <> BpLine(idx(j)) AND nl > 0 THEN
          Session.ClearBreakpoint BpFile(idx(j)), BpLine(idx(j))
          BpLine(idx(j)) = nl
          SendBreakpoint idx(j)
        END IF
      NEXT
    END IF
  NEXT
END SUB

SUB SyncAllBreakpoints
  DIM d AS INTEGER
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) THEN SyncBreakpointLines d
  NEXT
END SUB

' (a document opened: its breakpoints, where the program is paused, data tips)
SUB DebugDocOpened(d AS INTEGER)
  DIM i AS INTEGER
  CodeDoc(d).OnGutterClick = CodeGutterClick
  CodeDoc(d).OnHoverRequest = CodeHoverRequest
  FOR i = 0 TO BpCount - 1
    IF LCASE$(BpFile(i)) = LCASE$(DocFile(d)) THEN CodeDoc(d).AddMarker BpLine(i), BpKind(i)
  NEXT
  CodeDoc(d).DebugHover = Flag(RunState = "paused")
END SUB

SUB CodeGutterClick(LineNo AS INTEGER, Area AS STRING, Sender AS RCODEEDITOR)
  IF Area = "marker" THEN ToggleBreakpointAt Sender.Tag, LineNo
END SUB

' ---- the Breakpoints pane ------------------------------------------------------------------

FUNCTION BreakpointText(i AS INTEGER) AS STRING
  DIM t AS STRING
  t = FileTitle(BpFile(i)) + ":" + Num(BpLine(i))
  IF BpOn(i) = 0 THEN t = t + "  (off)"
  IF BpNever(i) AND BpOn(i) THEN t = t + "  (no code here: never stops)"
  IF BpCond(i) <> "" THEN t = t + "  when " + BpCond(i)
  IF BpHit(i) <> "" THEN t = t + "  hit " + BpHit(i)
  IF BpLog(i) <> "" THEN t = t + "  log " + CHR$(34) + BpLog(i) + CHR$(34)
  Result = t
END FUNCTION

SUB FillBreakpoints
  DIM i AS INTEGER
  BpTree.Clear
  FOR i = 0 TO BpCount - 1
    BpTree.AddItems BreakpointText(i)
  NEXT
  IF BpCount = 0 THEN BpPicked = -1
  IF BpPicked >= 0 THEN BpTree.ItemIndex = BpPicked
  ShowPickedBreakpoint
  IF BpCount > 0 THEN Dock.PaneTitle("bppanel", "Breakpoints (" + Num(BpCount) + ")") ELSE Dock.PaneTitle("bppanel", "Breakpoints")
END SUB

' (the selected breakpoint's rules in the edit strip)
SUB ShowPickedBreakpoint
  BpFilling = 1
  IF BpPicked >= 0 AND BpPicked < BpCount THEN
    BpCondEdit.Text = BpCond(BpPicked)
    BpHitEdit.Text = BpHit(BpPicked)
    BpLogEdit.Text = BpLog(BpPicked)
    BpOnCheck.Checked = BpOn(BpPicked)
    BpStrip.Enabled = 1
  ELSE
    BpCondEdit.Text = "": BpHitEdit.Text = "": BpLogEdit.Text = ""
    BpOnCheck.Checked = 0
    BpStrip.Enabled = 0
  END IF
  BpFilling = 0
END SUB

SUB BpTreeClick
  BpPicked = BpTree.ItemIndex
  ShowPickedBreakpoint
END SUB

' (double click / Enter: its line in the editor)
SUB BpTreeOpen
  DIM i AS INTEGER
  i = BpTree.ItemIndex
  IF i < 0 OR i >= BpCount THEN EXIT SUB
  ShowLocation BpFile(i), BpLine(i), ""
END SUB

SUB BpTreeKey(Key AS INTEGER, Shift AS INTEGER)
  DIM i AS INTEGER
  i = BpTree.ItemIndex
  IF i < 0 OR i >= BpCount THEN EXIT SUB
  SELECT CASE Key
    CASE 46, 8
      ' (Delete: gone)
      RemoveBreakpoint i
      FillBreakpoints
    CASE 32
      ' (Space: on / off)
      ChangeBreakpoint i, BpCond(i), BpHit(i), BpLog(i), 1 - BpOn(i)
    CASE 13
      BpTreeOpen
  END SELECT
END SUB

' (the strip's edits: Enter applies them; the check box at once)
SUB BpEditKey(Key AS INTEGER, Shift AS INTEGER)
  IF Key = 13 THEN ApplyBreakpointEdits
END SUB

SUB BpOnClicked
  IF BpFilling = 0 THEN ApplyBreakpointEdits
END SUB

SUB ApplyBreakpointEdits
  IF BpPicked < 0 OR BpPicked >= BpCount THEN EXIT SUB
  ChangeBreakpoint BpPicked, BpCondEdit.Text, BpHitEdit.Text, BpLogEdit.Text, BpOnCheck.Checked
  SetState RunState, "Breakpoint " + BreakpointText(BpPicked)
END SUB

' Debug > Edit Breakpoint (Shift+F9): the caret line's breakpoint (made if
' there's none) in the Breakpoints pane, its condition ready to type.
SUB EditBreakpointAtCaret
  DIM d AS INTEGER
  d = ActiveCode
  IF d < 0 THEN EXIT SUB
  SyncBreakpointLines d
  BpPicked = FindBreakpoint(DocFile(d), CodeDoc(d).CaretLine)
  IF BpPicked < 0 THEN BpPicked = AddBreakpoint(DocFile(d), CodeDoc(d).CaretLine)
  FillBreakpoints
  Dock.FocusPane("bppanel")
  BpCondEdit.SetFocus
END SUB

' ---- where the program is ------------------------------------------------------------------

' The debugger's marks off every document.
SUB ClearPausedMarks
  DIM d AS INTEGER
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) THEN
      CodeDoc(d).ClearMarkers "current"
      CodeDoc(d).ClearMarkers "frame"
      CodeDoc(d).ClearMarkers "exception"
      CodeDoc(d).DebugHover = Flag(RunState = "paused")
    END IF
  NEXT
END SUB

' A file's line shown: the file opened if it isn't, its code in front, the
' line in view with the caret on it, and a marker of Kind there (none: "").
SUB ShowLocation(InFile AS STRING, LineNo AS INTEGER, Kind AS STRING)
  DIM p AS STRING, d AS INTEGER
  p = DebugPath(InFile)
  IF p = "" OR LineNo < 1 THEN EXIT SUB
  d = DocOf(p)
  IF d < 0 AND FILEEXISTS(p) THEN
    OpenDocument p
    d = DocOf(p)
  END IF
  IF d < 0 THEN EXIT SUB
  Dock.FocusPane(DocPane(d))
  CodeDoc(d).GotoLine LineNo - 1
  EditorFacts d
  ' (the stop's message at the line's end: a run-time error's; a pause's
  ' while the program waits for its events; a condition that failed)
  IF (Kind = "exception" OR Kind = "current") AND Session.StopMessage <> "" THEN
    CodeDoc(d).AddMarker LineNo, Kind, Session.StopMessage
  ELSEIF Kind <> "" THEN
    CodeDoc(d).AddMarker LineNo, Kind
  END IF
END SUB

' ---- running ------------------------------------------------------------------------------------

SUB StartProgram(WithDebugger AS INTEGER)
  IF ProjectOpen = 0 OR RunState = "running" OR RunState = "paused" THEN EXIT SUB
  ' (F5 to the first form: docs/studio-wow.md PERF-8)
  RunStarted = TIMER
  PerfForm = -1
  ' (VB's way: what is being run is saved first)
  SaveAll
  AnalyzeActive
  IF Lang.ErrorCount > 0 THEN
    SetState "error", "Can't run: " + Num(Lang.ErrorCount) + " errors. The first is selected."
    OutputBox.Page = "problems"
    Dock.ShowPane("outputbox")
    GoToFirstProblem
    EXIT SUB
  END IF
  SyncAllBreakpoints
  ResetNeverBreakpoints
  OutputBox.Clear "output"
  OutputBox.Page = "output"
  Dock.ShowPane("outputbox")
  ClearDebugPanes
  Session.Program = ProjectMain
  Session.Debug = WithDebugger
  Session.BreakOnError = Flag(WithDebugger <> 0 AND BreakOnErrors <> 0)
  IF Session.Start THEN
    SetState "running", "Running " + FileTitle(ProjectMain)
  ELSE
    OutputLine "Can't run " + ProjectMain + ": " + Session.Error
    SetState "error", "Can't run " + FileTitle(ProjectMain)
  END IF
END SUB

' (Run with errors: the editor at the first one — docs/studio-wow.md RUN-3)
SUB GoToFirstProblem
  DIM d AS INTEGER, text AS STRING, row AS STRING, i AS INTEGER, n AS INTEGER
  d = ActiveCode
  IF d < 0 THEN EXIT SUB
  text = Lang.Diagnostics(DocFile(d))
  n = TALLY(text, CHR$(10))
  FOR i = 1 TO n
    row = FIELD$(text, CHR$(10), i)
    IF FIELD$(row, CHR$(9), 1) = "error" THEN
      ShowLocation FIELD$(row, CHR$(9), 5), VAL(FIELD$(row, CHR$(9), 2)), ""
      IF FIELD$(row, CHR$(9), 5) = "" THEN
        Dock.FocusPane(DocPane(d))
        CodeDoc(d).GotoLine VAL(FIELD$(row, CHR$(9), 2)) - 1
      END IF
      EXIT SUB
    END IF
  NEXT
END SUB

' (a new run places every breakpoint again: the code may have changed)
SUB ResetNeverBreakpoints
  DIM i AS INTEGER, any AS INTEGER
  FOR i = 0 TO BpCount - 1
    IF BpNever(i) THEN
      UnmarkBreakpoint i
      BpNever(i) = 0
      MarkBreakpoint i
      any = 1
    END IF
  NEXT
  IF any THEN FillBreakpoints
END SUB

SUB StopProgram
  IF RunState = "running" OR RunState = "paused" THEN Session.Stop
END SUB

' Run > Run in Browser: the program's web build served on this machine and
' opened in the default browser (the desktop's; on the web Studio is in
' the browser: Run runs it here).
SUB RunInBrowser
  DIM why AS STRING
  IF ProjectOpen = 0 THEN EXIT SUB
  SaveAll
  AnalyzeActive
  IF Lang.ErrorCount > 0 THEN
    SetState "error", "Can't run: " + Num(Lang.ErrorCount) + " errors. The first is selected."
    GoToFirstProblem
    EXIT SUB
  END IF
  Session.Program = ProjectMain
  Dock.ShowPane("outputbox")
  OutputLine "Run in Browser: building " + FileTitle(ProjectMain) + " for the web..."
  Session.RunInBrowser
  why = Session.Error
  IF why = "" THEN
    SetState RunState, "Opening " + FileTitle(ProjectMain) + " in the browser..."
  ELSEIF RunState = "stopped" OR RunState = "error" THEN
    ' (Studio is in the browser: it runs here, saying so)
    StartProgram 0
    OutputLine why
  ELSE
    OutputLine why
  END IF
END SUB

' Debug > Run to Cursor (Ctrl+F10): on to the caret's line (a program not
' running starts and stops there).
SUB RunToCursor
  DIM d AS INTEGER
  d = ActiveCode
  IF d < 0 THEN EXIT SUB
  IF RunState = "running" THEN EXIT SUB
  SyncBreakpointLines d
  Session.RunToCursor DocFile(d), CodeDoc(d).CaretLine
  IF RunState = "stopped" OR RunState = "error" THEN StartProgram 1
END SUB

' (the program's output as it comes; OutLineOpen: its last line isn't ended)
DIM OutLineOpen AS INTEGER

SUB ProgramOutput(Text AS STRING)
  OutputBox.Write Text
  IF Text <> "" THEN OutLineOpen = Flag(RIGHT$(Text, 1) <> CHR$(10))
END SUB

' (a line of Studio's own after the program's: on a line of its own)
SUB OutputAfterProgram(Text AS STRING)
  IF OutLineOpen THEN OutputBox.Write CHR$(10)
  OutLineOpen = 0
  OutputLine Text
END SUB

SUB ProgramStopped(Reason AS STRING, InFile AS STRING, LineNo AS INTEGER)
  DIM what AS STRING, kind AS STRING
  PausedFile = DebugPath(InFile)
  PausedLine = LineNo
  SELECT CASE Reason
    CASE "breakpoint": what = "Breakpoint"
    CASE "entry": what = "Paused at the start"
    CASE "exception": what = "Run-time error"
    CASE ELSE
      what = "Paused"
      ' (while the program waits for its events: a ShowModal, a dialog)
      IF Reason = "pause" AND Session.StopMessage <> "" THEN what = "Paused, " + LCASE$(Session.StopMessage)
  END SELECT
  IF Reason = "exception" THEN kind = "exception" ELSE kind = "current"
  SetState "paused", what + ": " + FileTitle(PausedFile) + ":" + Num(LineNo)
  ClearPausedMarks
  ShowLocation PausedFile, LineNo, kind
  IF Reason = "exception" THEN
    OutputAfterProgram FileTitle(PausedFile) + ":" + Num(LineNo) + ": " + Session.StopMessage
  END IF
  ' (a new stop: its references are new; Locals and the globals are in)
  RefsIn = "|" + Num(Session.LocalsRef) + "|" + Num(Session.GlobalsRef) + "|"
  FillCallStack
  FillVariables
  FillWatches
  Studio.SetFocus
  ' (a step's command to its line shown with its values: PerfStep)
  IF StepStarted > 0 THEN
    PerfStep = INT((TIMER - StepStarted) * 1000)
    StepStarted = 0
  END IF
END SUB

' (the program's first form of a run: the time since F5, PerfForm)
SUB ProgramFormShown(Id AS STRING)
  IF PerfForm < 0 AND RunStarted > 0 THEN PerfForm = INT((TIMER - RunStarted) * 1000)
END SUB

SUB ProgramContinued
  SetState "running", "Running " + FileTitle(ProjectMain)
  ClearPausedMarks
END SUB

SUB ProgramExited(Code AS INTEGER)
  OutputAfterProgram "[" + FileTitle(ProjectMain) + " ended, exit code " + Num(Code) + "]"
  IF Code = 0 THEN SetState "stopped", "Ready" ELSE SetState "error", "Ended with exit code " + Num(Code)
  ClearPausedMarks
  ClearDebugPanes
END SUB

' ---- Variables and Watch: trees of values ----------------------------------------------------
' A value is "name = value": an array, a TYPE, an object or a component
' (a handler's Sender) has children, fetched when it's opened (OnVariables)
' and kept open from stop to stop.

FUNCTION RefIn(Ref AS INTEGER) AS INTEGER
  Result = Flag(INSTR(RefsIn, "|" + Num(Ref) + "|") > 0)
END FUNCTION

FUNCTION IsOpen(Paths AS STRING, Path AS STRING) AS INTEGER
  Result = Flag(INSTR(Paths, "|" + Path + "|") > 0)
END FUNCTION

FUNCTION ValueText(Nm AS STRING, Value AS STRING) AS STRING
  Result = Nm + " = " + Value
END FUNCTION

' (Tree w: 0 Variables, 1 Watch)
SUB TreeAddChild(w AS INTEGER, Parent AS INTEGER, Text AS STRING)
  IF w = 0 THEN VarsTree.AddChildItems Parent, Text ELSE WatchTree.AddChildItems Parent, Text
END SUB

FUNCTION TreeCount(w AS INTEGER) AS INTEGER
  IF w = 0 THEN Result = VarsTree.ItemCount ELSE Result = WatchTree.ItemCount
END FUNCTION

SUB TreeItemData(w AS INTEGER, i AS INTEGER, Ref AS INTEGER, Path AS STRING)
  IF i > MAXVARITEMS THEN EXIT SUB
  IF w = 0 THEN
    VarItemRef(i) = Ref: VarItemPath(i) = Path
    IF Ref > 0 THEN VarsTree.Item(i).HasChildren = 1
  ELSE
    WatchItemRef(i) = Ref: WatchItemPath(i) = Path
    IF Ref > 0 THEN WatchTree.Item(i).HasChildren = 1
  END IF
END SUB

' The children of reference Ref under item Parent of tree w, depth first (a
' child is added after its parent's whole subtree: depth first keeps that
' the end of the list); an open child's children too — fetched first when
' this stop hasn't yet.
SUB AddValueChildren(w AS INTEGER, Parent AS INTEGER, Ref AS INTEGER, Path AS STRING, Depth AS INTEGER)
  DIM rows AS STRING, n AS INTEGER, k AS INTEGER, row AS STRING, nm AS STRING, cref AS INTEGER, cpath AS STRING, item AS INTEGER, opened AS STRING
  IF Depth > 8 THEN EXIT SUB
  rows = Session.Variables(Ref)
  n = TALLY(rows, CHR$(10))
  IF w = 0 THEN opened = VarsOpen ELSE opened = WatchOpen
  FOR k = 1 TO n
    IF TreeCount(w) >= MAXVARITEMS THEN EXIT SUB
    row = FIELD$(rows, CHR$(10), k)
    nm = FIELD$(row, CHR$(9), 1)
    cref = VAL(FIELD$(row, CHR$(9), 4))
    cpath = Path + "/" + nm
    TreeAddChild w, Parent, ValueText(nm, FIELD$(row, CHR$(9), 2))
    item = TreeCount(w) - 1
    TreeItemData w, item, cref, cpath
    IF cref > 0 AND IsOpen(opened, cpath) THEN
      IF RefIn(cref) THEN
        AddValueChildren w, item, cref, cpath, Depth + 1
      ELSE
        Session.Expand cref
      END IF
    END IF
  NEXT
END SUB

' (after a tree is filled: what was open, opened again)
SUB ReopenTree(w AS INTEGER)
  DIM i AS INTEGER, opened AS STRING
  IF w = 0 THEN opened = VarsOpen ELSE opened = WatchOpen
  FOR i = 0 TO TreeCount(w) - 1
    IF i > MAXVARITEMS THEN EXIT SUB
    IF w = 0 THEN
      IF IsOpen(opened, VarItemPath(i)) THEN VarsTree.Expand i, 0
    ELSE
      IF IsOpen(opened, WatchItemPath(i)) THEN WatchTree.Expand i, 0
    END IF
  NEXT
END SUB

SUB FillVariables
  DIM top AS INTEGER, root AS INTEGER
  TreeFilling = 1
  top = VarsTree.TopIndex
  VarsTree.Clear
  IF RunState <> "paused" THEN
    VarsTree.AddItems "Not paused: F9 sets a breakpoint, F5 runs."
    TreeItemData 0, 0, 0, ""
    TreeFilling = 0
    EXIT SUB
  END IF
  VarsTree.AddItems "Locals"
  TreeItemData 0, 0, 0, "L"
  root = 0
  IF Session.LocalsRef > 0 THEN AddValueChildren 0, root, Session.LocalsRef, "L", 0
  IF VarsTree.ItemCount = 1 THEN
    VarsTree.AddChildItems 0, "(none)"
    TreeItemData 0, 1, 0, "L/"
  END IF
  VarsTree.AddItems "Globals"
  root = VarsTree.ItemCount - 1
  TreeItemData 0, root, 0, "G"
  AddValueChildren 0, root, Session.GlobalsRef, "G", 0
  ReopenTree 0
  IF top < VarsTree.ItemCount THEN VarsTree.TopIndex = top
  TreeFilling = 0
END SUB

SUB FillWatches
  DIM rows AS STRING, n AS INTEGER, k AS INTEGER, row AS STRING, expr AS STRING, value AS STRING, wref AS INTEGER, item AS INTEGER
  TreeFilling = 1
  WatchTree.Clear
  rows = Session.WatchValues
  n = TALLY(rows, CHR$(10))
  FOR k = 1 TO n
    row = FIELD$(rows, CHR$(10), k)
    expr = FIELD$(row, CHR$(9), 1)
    value = FIELD$(row, CHR$(9), 2)
    wref = VAL(FIELD$(row, CHR$(9), 4))
    IF RunState <> "paused" THEN value = "(not paused)"
    WatchTree.AddItems ValueText(expr, value)
    item = WatchTree.ItemCount - 1
    TreeItemData 1, item, wref, "W" + Num(k)
    IF RunState = "paused" AND wref > 0 AND IsOpen(WatchOpen, "W" + Num(k)) THEN
      IF RefIn(wref) THEN AddValueChildren 1, item, wref, "W" + Num(k), 0 ELSE Session.Expand wref
    END IF
  NEXT
  ReopenTree 1
  TreeFilling = 0
END SUB

' A value opened by the user: its children fetched (then filled in: OnVariables).
SUB VarsExpanding(Index AS INTEGER, AllowExpansion AS INTEGER)
  IF TreeFilling OR Index < 0 OR Index > MAXVARITEMS THEN EXIT SUB
  IF IsOpen(VarsOpen, VarItemPath(Index)) = 0 THEN VarsOpen = VarsOpen + "|" + VarItemPath(Index) + "|"
  IF VarItemRef(Index) > 0 THEN
    IF RefIn(VarItemRef(Index)) THEN FillVariables ELSE Session.Expand VarItemRef(Index)
  END IF
END SUB

SUB VarsCollapsed(Index AS INTEGER)
  IF TreeFilling OR Index < 0 OR Index > MAXVARITEMS THEN EXIT SUB
  VarsOpen = REPLACESUBSTR$(VarsOpen, "|" + VarItemPath(Index) + "|", "")
END SUB

SUB WatchExpanding(Index AS INTEGER, AllowExpansion AS INTEGER)
  IF TreeFilling OR Index < 0 OR Index > MAXVARITEMS THEN EXIT SUB
  IF IsOpen(WatchOpen, WatchItemPath(Index)) = 0 THEN WatchOpen = WatchOpen + "|" + WatchItemPath(Index) + "|"
  IF WatchItemRef(Index) > 0 THEN
    IF RefIn(WatchItemRef(Index)) THEN FillWatches ELSE Session.Expand WatchItemRef(Index)
  END IF
END SUB

SUB WatchCollapsed(Index AS INTEGER)
  IF TreeFilling OR Index < 0 OR Index > MAXVARITEMS THEN EXIT SUB
  WatchOpen = REPLACESUBSTR$(WatchOpen, "|" + WatchItemPath(Index) + "|", "")
END SUB

' The session's news: a value's children in (Ref), or the frame's locals
' and the watches fetched again (0: a frame picked, a watch added, a
' statement run in Immediate).
SUB SessionVariables(Ref AS INTEGER)
  IF RunState <> "paused" THEN EXIT SUB
  IF Ref = 0 THEN
    RefsIn = "|" + Num(Session.LocalsRef) + "|" + Num(Session.GlobalsRef) + "|"
  ELSEIF RefIn(Ref) = 0 THEN
    RefsIn = RefsIn + Num(Ref) + "|"
  END IF
  FillVariables
  FillWatches
END SUB

' ---- Watch ------------------------------------------------------------------------------------

SUB AddWatch(Expr AS STRING)
  DIM e AS STRING
  e = LTRIM$(RTRIM$(Expr))
  IF e = "" THEN EXIT SUB
  Session.Watches = Session.Watches + e + CHR$(10)
  FillWatches
  Dock.ShowPane("watchpanel")
  DebugStateChanged
END SUB

SUB WatchEditKey(Key AS INTEGER, Shift AS INTEGER)
  IF Key = 13 THEN
    AddWatch WatchEdit.Text
    WatchEdit.Text = ""
  END IF
END SUB

' (Delete on a watch: gone)
SUB WatchTreeKey(Key AS INTEGER, Shift AS INTEGER)
  DIM i AS INTEGER, w AS STRING, n AS INTEGER, k AS INTEGER, acc AS STRING, at AS INTEGER
  IF Key <> 46 AND Key <> 8 THEN EXIT SUB
  i = WatchTree.ItemIndex
  IF i < 0 OR i > MAXVARITEMS THEN EXIT SUB
  IF LEFT$(WatchItemPath(i), 1) <> "W" OR INSTR(WatchItemPath(i), "/") > 0 THEN EXIT SUB
  at = VAL(MID$(WatchItemPath(i), 2))
  w = Session.Watches
  n = TALLY(w, CHR$(10))
  FOR k = 1 TO n
    IF k <> at THEN acc = acc + FIELD$(w, CHR$(10), k) + CHR$(10)
  NEXT
  Session.Watches = acc
  WatchOpen = ""
  FillWatches
  DebugStateChanged
END SUB

' Debug > Add Watch: the selection in the code, or the expression under
' the caret; nothing there: the Watch pane's box, ready to type.
SUB AddWatchCommand
  DIM d AS INTEGER, e AS STRING
  d = ActiveCode
  IF d >= 0 THEN
    e = CodeDoc(d).SelText
    IF e = "" OR INSTR(e, CHR$(10)) > 0 THEN
      e = CodeDoc(d).WordAt(CodeDoc(d).CaretLine, CodeDoc(d).CaretColumn)
      IF NotAValue(e) THEN e = ""
    END IF
  END IF
  IF e <> "" THEN
    AddWatch e
  ELSE
    Dock.FocusPane("watchpanel")
    WatchEdit.SetFocus
  END IF
END SUB

' ---- Call Stack -----------------------------------------------------------------------------------

SUB FillCallStack
  DIM rows AS STRING, n AS INTEGER, k AS INTEGER, row AS STRING
  TreeFilling = 1
  StackTree.Clear
  FrameCount = 0
  rows = Session.StackTrace
  n = TALLY(rows, CHR$(10))
  FOR k = 1 TO n
    IF k > 64 THEN EXIT FOR
    row = FIELD$(rows, CHR$(10), k)
    FrameName(k - 1) = FIELD$(row, CHR$(9), 1)
    IF LEFT$(FrameName(k - 1), 2) = "__" THEN FrameName(k - 1) = "(the program)"
    FrameFile(k - 1) = DebugPath(FIELD$(row, CHR$(9), 2))
    FrameLine(k - 1) = VAL(FIELD$(row, CHR$(9), 3))
    StackTree.AddItems FrameName(k - 1) + "   " + FileTitle(FrameFile(k - 1)) + ":" + Num(FrameLine(k - 1))
    FrameCount = k
  NEXT
  IF FrameCount > 0 THEN StackTree.ItemIndex = Session.Frame
  TreeFilling = 0
END SUB

' A frame picked: its line shown (a caller's: marked "frame"), its locals
' and the watches in it (OnVariables(0)).
SUB StackPicked
  DIM f AS INTEGER
  IF TreeFilling THEN EXIT SUB
  f = StackTree.ItemIndex
  IF f < 0 OR f >= FrameCount OR RunState <> "paused" THEN EXIT SUB
  ClearFrameMarks
  IF f = 0 THEN
    ShowLocation FrameFile(0), FrameLine(0), ""
  ELSE
    ShowLocation FrameFile(f), FrameLine(f), "frame"
  END IF
  IF Session.Frame <> f THEN Session.Frame = f
END SUB

SUB ClearFrameMarks
  DIM d AS INTEGER
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) THEN CodeDoc(d).ClearMarkers "frame"
  NEXT
END SUB

SUB ClearDebugPanes
  RefsIn = ""
  FrameCount = 0
  TreeFilling = 1
  StackTree.Clear
  TreeFilling = 0
  FillVariables
  FillWatches
END SUB

' ---- Immediate: "? expr" prints, a statement runs in the paused frame ----------------------------

SUB ImmediateKey(Key AS INTEGER, Shift AS INTEGER)
  DIM n AS INTEGER, typed AS STRING
  n = ImmediateBox.LineCount
  IF n < 1 THEN EXIT SUB
  ' (Up / Down on the last line: the lines typed before)
  IF (Key = 38 OR Key = 40) AND ImmCount > 0 THEN
    IF Key = 38 THEN ImmAt = ImmAt - 1 ELSE ImmAt = ImmAt + 1
    IF ImmAt < 0 THEN ImmAt = 0
    IF ImmAt > ImmCount THEN ImmAt = ImmCount
    ImmediateBox.Line(n - 1) = ImmHistory(ImmAt)
    ImmediateBox.SelStart = LEN(ImmediateBox.Text)
    Key = 0
    EXIT SUB
  END IF
  IF Key <> 13 THEN EXIT SUB
  typed = ImmediateBox.Line(n - 1)
  IF LTRIM$(typed) = "" THEN EXIT SUB
  IF ImmCount < 50 THEN
    ImmHistory(ImmCount) = typed
    ImmCount = ImmCount + 1
  END IF
  ImmAt = ImmCount
  IF RunState = "paused" THEN
    ImmEvalId = Session.Evaluate(typed, "repl")
  ELSE
    ImmediateBox.AddStrings "(the program isn't paused: F9 sets a breakpoint, Debug > Pause stops it)"
  END IF
END SUB

' (an evaluation's answer: the Immediate window's, or a data tip's)
SUB SessionEvaluated(Id AS INTEGER, Answer AS STRING)
  IF Id = ImmEvalId THEN
    IF Answer <> "" THEN ImmediateBox.AddStrings Answer
  ELSEIF Id = HoverEvalId THEN
    IF LEFT$(Answer, 6) <> "error:" AND HoverDoc >= 0 AND HoverDoc < DocCount THEN
      IF DocOpen(HoverDoc) THEN CodeDoc(HoverDoc).ShowHover "```" + CHR$(10) + HoverExpr + " = " + Answer + CHR$(10) + "```"
    END IF
  END IF
END SUB

' ---- data tips: a resting mouse's value while paused -----------------------------------------------

' (a name the editor found — WordAt — that isn't a value to show: none, a
' number, a word of the language)
FUNCTION NotAValue(Name AS STRING) AS INTEGER
  Result = 1
  IF Name = "" THEN EXIT FUNCTION
  IF INSTR("0123456789", LEFT$(Name, 1)) > 0 THEN EXIT FUNCTION
  IF INSTR("|IF|THEN|ELSE|ELSEIF|END|SUB|FUNCTION|DIM|AS|FOR|TO|NEXT|DO|LOOP|WHILE|WEND|SELECT|CASE|AND|OR|NOT|MOD|XOR|CREATE|PRINT|RETURN|EXIT|STEP|UNTIL|INTEGER|STRING|DOUBLE|SINGLE|LONG|BYTE|WORD|DWORD|SHORT|CALL|GOTO|GOSUB|", "|" + UCASE$(Name) + "|") > 0 THEN EXIT FUNCTION
  Result = 0
END FUNCTION

SUB CodeHoverRequest(LineNo AS INTEGER, ColNo AS INTEGER, Sender AS RCODEEDITOR)
  DIM d AS INTEGER, e AS STRING
  IF RunState <> "paused" THEN EXIT SUB
  d = Sender.Tag
  ' (the dotted name there — Edit1.Text — lines and columns from 1, as
  ' OnHoverRequest gives them; a number or a word of the language isn't a value)
  e = Sender.WordAt(LineNo, ColNo)
  IF NotAValue(e) THEN EXIT SUB
  HoverDoc = d
  HoverExpr = e
  HoverEvalId = Session.Evaluate("? " + e)
END SUB

' ---- the commands ------------------------------------------------------------------------------
' True when Id was one of Run's or Debug's (and it ran).
FUNCTION DebugCommand(Id AS STRING) AS INTEGER
  DIM d AS INTEGER
  d = ActiveCode
  Result = 1
  IF LEFT$(Id, 10) = "debug.step" AND RunState = "paused" THEN StepStarted = TIMER
  SELECT CASE Id
    CASE "run.start"
      IF RunState = "paused" THEN Session.Continue ELSE StartProgram 1
    CASE "run.withoutDebugging": StartProgram 0
    CASE "run.stop": StopProgram
    CASE "run.restart"
      StopProgram
      StartProgram 1
    CASE "run.browser": RunInBrowser
    CASE "run.continue": IF RunState = "paused" THEN Session.Continue
    CASE "run.pause": IF RunState = "running" THEN Session.Pause
    CASE "debug.stepInto"
      ' (not running: it starts, stopping at the first line — VB6's F8)
      IF RunState = "paused" THEN
        Session.StepIn
      ELSEIF RunState <> "running" THEN
        RunFromEntry
      END IF
    CASE "debug.stepOver"
      IF RunState = "paused" THEN
        Session.StepOver
      ELSEIF RunState <> "running" THEN
        RunFromEntry
      END IF
    CASE "debug.stepOut": IF RunState = "paused" THEN Session.StepOut
    CASE "debug.runToCursor": RunToCursor
    CASE "debug.toggleBreakpoint"
      IF d >= 0 THEN ToggleBreakpointAt d, CodeDoc(d).CaretLine
    CASE "debug.editBreakpoint": EditBreakpointAtCaret
    CASE "debug.clearBreakpoints": ClearAllBreakpoints
    CASE "debug.addWatch": AddWatchCommand
    CASE "debug.breakOnErrors"
      BreakOnErrors = 1 - BreakOnErrors
      CheckCmd "debug.breakOnErrors", BreakOnErrors
      IF BreakOnErrors THEN SetState RunState, "The debugger stops at run-time errors" ELSE SetState RunState, "Run-time errors end the program"
    CASE "view.variables": Dock.FocusPane("varstree")
    CASE "view.watch": Dock.FocusPane("watchpanel")
    CASE "view.callStack": Dock.FocusPane("stacktree")
    CASE "view.breakpoints": Dock.FocusPane("bppanel")
    CASE ELSE: Result = 0
  END SELECT
END FUNCTION

' (F11 / F10 before running: stopped at the program's first statement)
SUB RunFromEntry
  Session.StopOnEntry = 1
  StartProgram 1
  Session.StopOnEntry = 0
END SUB

' ---- the scripted flows' steps (tests/studio_flows.mjs; the --do list) ---------------------------
'   line:<N>          the caret to line N of the active code (then F9, Ctrl+F10 …)
'   watch:<expr>      a watch added, as typed in the Watch pane
'   bpcond:<expr>, bphit:<rule>, bplog:<text>, bpoff:   the picked breakpoint's
'                     rules, as typed in the Breakpoints pane (and Enter)
'   hover:<L>:<C>     a resting mouse's data tip at line L, column C of the active code
'   frame:<N>         a frame picked in the Call Stack
' True when Stp was one of these.
FUNCTION DebugStep(Stp AS STRING) AS INTEGER
  DIM k AS INTEGER, verb AS STRING, arg AS STRING, d AS INTEGER
  k = INSTR(Stp, ":")
  IF k < 2 THEN EXIT FUNCTION
  verb = LEFT$(Stp, k - 1)
  arg = MID$(Stp, k + 1)
  d = ActiveCode
  Result = 1
  SELECT CASE verb
    CASE "line"
      IF d >= 0 THEN
        Dock.FocusPane(DocPane(d))
        CodeDoc(d).GotoLine VAL(arg) - 1
      END IF
    CASE "watch"
      WatchEdit.Text = arg
      WatchEditKey 13, 0
    CASE "bpcond", "bphit", "bplog", "bpoff"
      IF BpPicked >= 0 THEN
        IF verb = "bpcond" THEN BpCondEdit.Text = arg
        IF verb = "bphit" THEN BpHitEdit.Text = arg
        IF verb = "bplog" THEN BpLogEdit.Text = arg
        IF verb = "bpoff" THEN BpOnCheck.Checked = 0
        ApplyBreakpointEdits
      END IF
    CASE "hover"
      IF d >= 0 THEN CodeHoverRequest VAL(FIELD$(arg, ":", 1)), VAL(FIELD$(arg, ":", 2)), CodeDoc(d)
    CASE "frame"
      StackTree.ItemIndex = VAL(arg)
      StackPicked
    CASE "expand"
      ' (expand:<path> — a value in Variables opened, as a click on its + does)
      FOR k = 0 TO VarsTree.ItemCount - 1
        IF k > MAXVARITEMS THEN EXIT FOR
        IF VarItemPath(k) = arg THEN
          VarsExpanding k, 1
          VarsTree.Expand k, 0
          EXIT FOR
        END IF
      NEXT
    CASE "perf"
      ' (perf:report — the run's and the steps' times, docs/studio-wow.md §3.15)
      OutputLine "[perf] f5-form=" + Num(PerfForm) + " step=" + Num(PerfStep)
    CASE ELSE: Result = 0
  END SELECT
END FUNCTION

' ---- set-up -------------------------------------------------------------------------------------

SUB DebugTree(t AS QTREEVIEW, Title AS STRING)
  t.Parent = Studio
  t.Width = 440: t.Height = 190
  t.ReadOnly = 1
  t.HideSelection = 0
  t.AccessibleName = Title
END SUB

' (the debugger's group beside Output as wide as Output, so its four tabs
' have room: their sizes in the layout's text)
SUB WidenDebugPanes
  DIM lay AS STRING, n AS INTEGER, k AS INTEGER, ln AS STRING, sp AS INTEGER, acc AS STRING, lead AS STRING
  DIM total AS INTEGER, outAt AS INTEGER, varsAt AS INTEGER
  lay = Dock.SaveLayout
  n = TALLY(lay, CHR$(10))
  FOR k = 1 TO n + 1
    ln = FIELD$(lay, CHR$(10), k)
    IF INSTR(ln, " tabs ") > 0 AND INSTR(ln, " outputbox") > 0 THEN outAt = k: total = total + VAL(LTRIM$(ln))
    IF INSTR(ln, " tabs ") > 0 AND INSTR(ln, " varstree") > 0 THEN varsAt = k: total = total + VAL(LTRIM$(ln))
  NEXT
  IF outAt = 0 OR varsAt = 0 OR total < 600 THEN EXIT SUB
  FOR k = 1 TO n + 1
    ln = FIELD$(lay, CHR$(10), k)
    IF k = outAt OR k = varsAt THEN
      lead = LEFT$(ln, LEN(ln) - LEN(LTRIM$(ln)))
      sp = INSTR(LTRIM$(ln), " ")
      IF k = varsAt THEN ln = lead + Num(total * 3 \ 5) + MID$(LTRIM$(ln), sp) ELSE ln = lead + Num(total - total * 3 \ 5) + MID$(LTRIM$(ln), sp)
    END IF
    IF k <= n THEN acc = acc + ln + CHR$(10) ELSE acc = acc + ln
  NEXT
  Dock.LoadLayout(acc)
END SUB

' The panes beside Output (Variables, Watch, Call Stack, Breakpoints), the
' session's events.
SUB SetUpDebug
  BreakOnErrors = 1
  CheckCmd "debug.breakOnErrors", 1
  VarsOpen = "|L|G|"
  BpPicked = -1
  ImmEvalId = -1
  HoverEvalId = -1
  Session.OnOutput = ProgramOutput
  Session.OnStopped = ProgramStopped
  Session.OnContinue = ProgramContinued
  Session.OnExit = ProgramExited
  Session.OnVariables = SessionVariables
  Session.OnEvaluate = SessionEvaluated
  Session.OnFormShown = ProgramFormShown
  Session.OnBreakpointPlaced = BreakpointPlaced
  PerfForm = -1
  PerfStep = -1
  ' (RAPIDR_STUDIO_PERF: the moment Studio is up and waiting, printed for
  ' tests/studio_perf.mjs — PERF-1's cold start)
  IF ENVIRON$("RAPIDR_STUDIO_PERF") <> "" THEN PerfReady.Enabled = 1

  DebugTree VarsTree, "Variables"
  VarsTree.OnExpanding = VarsExpanding
  VarsTree.OnCollapsed = VarsCollapsed

  WatchPanel.Parent = Studio
  WatchPanel.Width = 440: WatchPanel.Height = 190
  WatchPanel.BevelOuter = 0
  WatchPanel.AccessibleName = "Watch"
  ' (a strip at the top: "Add" and the box an expression is typed in)
  WatchStrip.Parent = WatchPanel
  WatchStrip.Align = 1
  WatchStrip.Height = 30
  WatchStrip.BevelOuter = 0
  WatchLabel.Parent = WatchStrip
  WatchLabel.Left = 6: WatchLabel.Top = 7: WatchLabel.Width = 88
  WatchLabel.Caption = "&Add a watch:"
  WatchEdit.Parent = WatchStrip
  WatchEdit.Left = 96: WatchEdit.Top = 4: WatchEdit.Width = 260
  WatchEdit.Hint = "Add a watch: an expression, then Enter"
  WatchEdit.AccessibleName = "Add a watch"
  WatchEdit.OnKeyDown = WatchEditKey
  DebugTree WatchTree, "Watches"
  WatchTree.Parent = WatchPanel
  WatchTree.Align = 5
  WatchTree.OnExpanding = WatchExpanding
  WatchTree.OnCollapsed = WatchCollapsed
  WatchTree.OnKeyDown = WatchTreeKey

  DebugTree StackTree, "Call Stack"
  StackTree.ShowRoot = 0
  StackTree.ShowLines = 0
  StackTree.ShowButtons = 0
  StackTree.OnChange = StackPicked

  BpPanel.Parent = Studio
  BpPanel.Width = 440: BpPanel.Height = 190
  BpPanel.BevelOuter = 0
  BpPanel.AccessibleName = "Breakpoints"
  BpStrip.Parent = BpPanel
  BpStrip.Align = 2
  BpStrip.Height = 30
  BpStrip.BevelOuter = 0
  BpCondLabel.Parent = BpStrip
  BpCondLabel.Left = 6: BpCondLabel.Top = 7: BpCondLabel.Width = 34
  BpCondLabel.Caption = "&When"
  BpCondEdit.Parent = BpStrip
  BpCondEdit.Left = 42: BpCondEdit.Top = 4: BpCondEdit.Width = 130
  BpCondEdit.Hint = "A condition: it stops when this is true (clicks > 1)"
  BpCondEdit.AccessibleName = "Condition"
  BpCondEdit.OnKeyDown = BpEditKey
  BpHitLabel.Parent = BpStrip
  BpHitLabel.Left = 180: BpHitLabel.Top = 7: BpHitLabel.Width = 22
  BpHitLabel.Caption = "&Hit"
  BpHitEdit.Parent = BpStrip
  BpHitEdit.Left = 204: BpHitEdit.Top = 4: BpHitEdit.Width = 50
  BpHitEdit.Hint = "A hit count: 3 (the third time), >= 3, % 3 (every third)"
  BpHitEdit.AccessibleName = "Hit count"
  BpHitEdit.OnKeyDown = BpEditKey
  BpLogLabel.Parent = BpStrip
  BpLogLabel.Left = 262: BpLogLabel.Top = 7: BpLogLabel.Width = 26
  BpLogLabel.Caption = "&Log"
  BpLogEdit.Parent = BpStrip
  BpLogEdit.Left = 290: BpLogEdit.Top = 4: BpLogEdit.Width = 116
  BpLogEdit.Hint = "A logpoint: prints this ({expr} shows a value) and goes on"
  BpLogEdit.AccessibleName = "Log message"
  BpLogEdit.OnKeyDown = BpEditKey
  BpOnCheck.Parent = BpStrip
  BpOnCheck.Left = 414: BpOnCheck.Top = 6: BpOnCheck.Width = 46
  BpOnCheck.Caption = "On"
  BpOnCheck.OnClick = BpOnClicked
  DebugTree BpTree, "Breakpoints"
  BpTree.Parent = BpPanel
  BpTree.Align = 5
  BpTree.ShowRoot = 0
  BpTree.ShowLines = 0
  BpTree.ShowButtons = 0
  BpTree.OnClick = BpTreeClick
  BpTree.OnDblClick = BpTreeOpen
  BpTree.OnKeyDown = BpTreeKey

  Dock.AddPane(VarsTree, "Variables", "right:outputbox", "components/variablesview")
  Dock.AddPane(WatchPanel, "Watch", "tab:varstree", "actions/watch")
  Dock.AddPane(StackTree, "Call Stack", "tab:varstree", "components/callstackview")
  Dock.AddPane(BpPanel, "Breakpoints", "tab:varstree", "components/breakpointlist")
  Dock.ShowPane("varstree")
  WidenDebugPanes
  FillVariables
  FillWatches
  FillBreakpoints
END SUB

' ---- kept with the project: its breakpoints and watches -----------------------------------
' In Studio's settings (the per-user store; the page's on the web) by the
' project's file, saved a moment after they change (as the layout is:
' workspace.inc) and when Studio closes: opening the project again brings
' them back, as Xcode, Delphi and VS Code do. Files relative to the
' project's folder; a row a breakpoint: file, line, on, condition, hit
' count, log message (tab-separated).

FUNCTION DebugStateName(Kind AS STRING) AS STRING
  Result = Kind + " " + LCASE$(REPLACESUBSTR$(ProjectFile, "\", "/"))
END FUNCTION

' (a field without the row's separators)
FUNCTION StateField(Text AS STRING) AS STRING
  Result = REPLACESUBSTR$(REPLACESUBSTR$(REPLACESUBSTR$(Text, CHR$(9), " "), CHR$(13), " "), CHR$(10), " ")
END FUNCTION

' (a file as the project names it: relative to its folder when inside it)
FUNCTION ProjectRelative(Path AS STRING) AS STRING
  DIM f AS STRING, root AS STRING
  f = REPLACESUBSTR$(Path, "\", "/")
  root = REPLACESUBSTR$(Proj.Folder, "\", "/")
  Result = f
  IF root = "" THEN EXIT FUNCTION
  IF RIGHT$(root, 1) <> "/" THEN root = root + "/"
  IF LCASE$(LEFT$(f, LEN(root))) = LCASE$(root) THEN Result = MID$(f, LEN(root) + 1)
END FUNCTION

' (something to keep changed: saved with the layout's timer)
SUB DebugStateChanged
  IF DebugLoading = 0 THEN LayoutChanged
END SUB

' (Reg open at Studio's key: SaveSettings)
SUB SaveDebugState
  DIM i AS INTEGER, t AS STRING
  IF ProjectOpen = 0 OR ProjectFile = "" THEN EXIT SUB
  FOR i = 0 TO BpCount - 1
    t = t + StateField(ProjectRelative(BpFile(i))) + CHR$(9) + Num(BpLine(i)) + CHR$(9) + Num(BpOn(i)) + CHR$(9) + StateField(BpCond(i)) + CHR$(9) + StateField(BpHit(i)) + CHR$(9) + StateField(BpLog(i)) + CHR$(10)
  NEXT
  Reg.WriteString(DebugStateName("Breakpoints"), t)
  Reg.WriteString(DebugStateName("Watches"), Session.Watches)
END SUB

' The project just opened: its breakpoints and watches as they were (the
' last project's gone). Not with --fresh (the tests' clean start).
SUB LoadDebugState
  DIM t AS STRING, w AS STRING, n AS INTEGER, k AS INTEGER, row AS STRING, i AS INTEGER, f AS STRING
  DebugLoading = 1
  DO WHILE BpCount > 0
    RemoveBreakpoint BpCount - 1
  LOOP
  Session.ClearBreakpoints
  BpPicked = -1
  IF SettingsRead THEN
    Reg.RootKey = HKCU
    IF Reg.KeyExists(STUDIO_KEY) THEN
      Reg.OpenKey(STUDIO_KEY, 0)
      IF Reg.ValueExists(DebugStateName("Breakpoints")) THEN t = Reg.ReadString(DebugStateName("Breakpoints"))
      IF Reg.ValueExists(DebugStateName("Watches")) THEN w = Reg.ReadString(DebugStateName("Watches"))
      Reg.CloseKey
    END IF
  END IF
  Session.Watches = w
  WatchOpen = ""
  n = TALLY(t, CHR$(10))
  FOR k = 1 TO n
    row = FIELD$(t, CHR$(10), k)
    f = FIELD$(row, CHR$(9), 1)
    IF f <> "" AND VAL(FIELD$(row, CHR$(9), 2)) > 0 THEN
      i = AddBreakpoint(ProjectPath(f), VAL(FIELD$(row, CHR$(9), 2)))
      IF i >= 0 THEN ChangeBreakpoint i, FIELD$(row, CHR$(9), 4), FIELD$(row, CHR$(9), 5), FIELD$(row, CHR$(9), 6), VAL(FIELD$(row, CHR$(9), 3))
    END IF
  NEXT
  FillBreakpoints
  FillWatches
  DebugLoading = 0
END SUB


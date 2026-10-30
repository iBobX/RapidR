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
DIM VarsTree AS RTreeView
DIM WatchPanel AS RPanel
DIM WatchStrip AS RPanel
DIM WatchLabel AS RLabel
DIM WatchEdit AS REdit
DIM WatchTree AS RTreeView
DIM StackTree AS RTreeView
DIM BpPanel AS RPanel
DIM BpTree AS RTreeView
DIM BpStrip AS RPanel
DIM BpCondLabel AS RLabel
DIM BpCondEdit AS REdit
DIM BpHitLabel AS RLabel
DIM BpHitEdit AS REdit
DIM BpLogLabel AS RLabel
DIM BpLogEdit AS REdit
DIM BpOnCheck AS RCheckBox

' The trees' items: what each shows (the variables' reference: its children,
' 0 none; its path, so what was open stays open from stop to stop; the
' expression that names its value — what Copy, Add Watch and setting it use;
' its type; for "… more", the reference and where the next page starts).
CONST MAXVARITEMS = 1500
DIM VarItemRef(MAXVARITEMS) AS INTEGER
DIM VarItemPath(MAXVARITEMS) AS STRING
DIM VarItemExpr(MAXVARITEMS) AS STRING
DIM VarItemKind(MAXVARITEMS) AS STRING
DIM VarItemMore(MAXVARITEMS) AS INTEGER
' (S11: what the registry knows of it — color, bool, const:…, choices:…,
' readonly — and its value as the debugger wrote it)
DIM VarItemHint(MAXVARITEMS) AS STRING
DIM VarItemValue(MAXVARITEMS) AS STRING
DIM WatchItemHint(MAXVARITEMS) AS STRING
DIM WatchItemValue(MAXVARITEMS) AS STRING
DIM WatchItemRef(MAXVARITEMS) AS INTEGER
DIM WatchItemPath(MAXVARITEMS) AS STRING
DIM WatchItemExpr(MAXVARITEMS) AS STRING
DIM WatchItemKind(MAXVARITEMS) AS STRING
DIM WatchItemMore(MAXVARITEMS) AS INTEGER
' (a value changed since the last stop: its colour in the trees, as the
' data tips mark it — the theme's "debug.changed": ApplyDebugColors)
DIM ChangedColor AS LONG

' ---- the data tip: the value under a resting mouse, as a tree -----------------------------
' Its rows as the last ShowDataTip showed them (a row's id is its number):
' its path (what's open stays open), the expression naming it, its type,
' its children's reference and how many, its value; a "… more" row's
' reference and next page's start.
CONST MAXTIPROWS = 400
DIM TipRowPath(MAXTIPROWS) AS STRING
DIM TipRowExpr(MAXTIPROWS) AS STRING
DIM TipRowKind(MAXTIPROWS) AS STRING
DIM TipRowValue(MAXTIPROWS) AS STRING
DIM TipRowRef(MAXTIPROWS) AS INTEGER
DIM TipRowMore(MAXTIPROWS) AS INTEGER
DIM TipRowCount AS INTEGER
DIM TipRows AS STRING
' (the expression under the mouse, its value, type and children; what is
' open; references still to fetch once the rows are made)
DIM TipExpr AS STRING
DIM TipValue AS STRING
DIM TipKind AS STRING
DIM TipChanged AS INTEGER
DIM TipRef AS INTEGER
DIM TipOpen AS STRING
DIM TipFetch AS STRING
DIM TipShown AS INTEGER
' (the tip's top value's columns, when it's a table; how many children it has)
DIM TipColumns AS STRING
DIM TipTotal AS INTEGER
' (the values data tips showed at the last stops, by expression: a changed
' one is marked)
DIM TipSeen AS STRING
DIM TipSeenNow AS STRING

' (Copy Value's and the text viewer's evaluations: their ids)
DIM CopyEvalId AS INTEGER
DIM ViewEvalId AS INTEGER
DIM ViewExpr AS STRING
DIM GotoEvalId AS INTEGER
' (Step Into Specific: the calls the line makes, picked from a menu)
DIM TargetMenu AS RPopupMenu
CONST MAXTARGETS = 12
DIM TargetItem(MAXTARGETS) AS RMenuItem
DIM TargetId(MAXTARGETS) AS INTEGER
DIM TargetCount AS INTEGER
' (a scripted flow's pick: into:<name>)
DIM TargetWanted AS STRING
' (the Variables and Watch panes' right-click menus)
DIM VarsMenu AS RPopupMenu
DIM WatchMenu AS RPopupMenu
DIM VarsMenuItem(4) AS RMenuItem
DIM WatchMenuItem(4) AS RMenuItem

' ---- the text viewer: a string's whole text -------------------------------------------
CREATE TextViewer AS RForm
  Caption = TR$("Text")
  Width = 560: Height = 400
  BorderStyle = 2
  CREATE TextViewerBox AS RMemo
    Left = 10: Top = 10: Width = 524: Height = 300
    ReadOnly = 1
    WordWrap = 1
    ScrollBars = 2
    AccessibleName = TR$("The text")
  END CREATE
  CREATE TextViewerInfo AS RLabel
    Left = 10: Top = 322: Width = 300: Caption = ""
  END CREATE
  CREATE TextViewerCopy AS RButton
    Left = 342: Top = 318: Width = 90: Caption = TR$("&Copy")
  END CREATE
  CREATE TextViewerClose AS RButton
    Left = 444: Top = 318: Width = 90: Caption = TR$("Close")
    Cancel = 1
    ModalResult = 2
  END CREATE
END CREATE
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
DIM ImmPending AS STRING
DIM HoverEvalId AS INTEGER
' (a value set from Variables or Watch: its expression, for the answer)
DIM SetVarId AS INTEGER
DIM SetVarExpr AS STRING
' ---- values set while paused (S11) --------------------------------------------------------------
' The item being set (tree w: 0 Variables, 1 Watch; its index), its value
' before (as an expression: what Undo sets back), the whole text asked for
' first (a STRING's: never the shortened one shown), a set that is an
' Undo's (not remembered again), one typed in Immediate (its answer there).
DIM EditW AS INTEGER, EditI AS INTEGER
DIM EditFullId AS INTEGER
DIM EditOld AS STRING
DIM EditFull AS STRING
' (the in-place edit open is a value's — EditItem —, not F2's own)
DIM EditingValue AS INTEGER
DIM SetVarOld AS STRING
' (a value set by a constant's name: said by it, not its number)
DIM SetVarShown AS STRING
DIM SetVarUndo AS INTEGER
DIM SetVarImm AS INTEGER
' (Immediate's `x = 5`: its place, the value typed, the old value asked for)
DIM ImmSetTarget AS STRING, ImmSetValue AS STRING
DIM ImmOldId AS INTEGER
' (what was set since the program paused, newest last: Ctrl+Z sets it back)
CONST MAXUNDO = 50
DIM UndoExpr(MAXUNDO) AS STRING
DIM UndoOld(MAXUNDO) AS STRING
DIM UndoCount AS INTEGER
' (a STRING's whole text, in its own window: long or of several lines)
CREATE ValueEditor AS RForm
  Caption = TR$("Set Value")
  Width = 560: Height = 400
  BorderStyle = 2
  CREATE ValueEditorBox AS RMemo
    Left = 10: Top = 10: Width = 524: Height = 300
    WordWrap = 1
    ScrollBars = 2
    AccessibleName = TR$("The new text")
  END CREATE
  CREATE ValueEditorInfo AS RLabel
    Left = 10: Top = 322: Width = 300: Caption = ""
  END CREATE
  CREATE ValueEditorSet AS RButton
    Left = 342: Top = 318: Width = 90: Caption = TR$("&Set")
  END CREATE
  CREATE ValueEditorCancel AS RButton
    Left = 444: Top = 318: Width = 90: Caption = TR$("Cancel")
    Cancel = 1
    ModalResult = 2
  END CREATE
END CREATE
' (a property's constants to pick from: a menu at the item)
CONST MAXCHOICES = 40
DIM ChoiceMenu AS RPopupMenu
DIM ChoiceItem(MAXCHOICES) AS RMenuItem
DIM ChoiceValue(MAXCHOICES) AS STRING
DIM ChoiceWanted AS STRING
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
' (the program printed something this run)
DIM RunOutputSeen AS INTEGER
DIM PerfStep AS INTEGER
DIM StepStarted AS DOUBLE
DIM PerfReady AS RTimer
PerfReady.Interval = 1
PerfReady.Enabled = 0
PerfReady.OnTimer = PerfReadyTick
' (the program's turn: a step that hasn't stopped within half a second —
' over a ShowModal, into the next event's handler — brings the program's
' windows to the front, as Continue does at once)
DIM StepRaise AS RTimer
StepRaise.Interval = 500
StepRaise.Enabled = 0
StepRaise.OnTimer = StepRaiseTick
' (POLISH-2) After a pause the program runs on: once it has run a moment —
' a step that stops again at once doesn't flicker —, Call Stack, Variables
' and Watch say it's running (the last pause's values are no longer the
' program's) and fill again at the next pause, Variables where it was.
DIM StaleTimer AS RTimer
StaleTimer.Interval = 300
StaleTimer.Enabled = 0
StaleTimer.OnTimer = StaleTick
DIM DebugStale AS INTEGER
DIM StaleVarsTop AS INTEGER
DIM StaleVarsPicked AS STRING
' (this run's first window has been brought to the front)
DIM RunRaised AS INTEGER
' (the first window's raise, tried again until the system knows the new
' process as an app — a moment after its window shows)
DIM RaiseRetry AS RTimer
RaiseRetry.Interval = 150
RaiseRetry.Enabled = 0
RaiseRetry.OnTimer = RaiseRetryTick
DIM RaiseTries AS INTEGER

SUB StepRaiseTick
  StepRaise.Enabled = 0
  IF RunState = "running" THEN ShowProgram 0
END SUB

SUB StaleTick
  StaleTimer.Enabled = 0
  IF RunState = "running" THEN ShowRunningPanes
END SUB

' (the program runs: the panes say so — Variables' place kept for the next
' pause)
SUB ShowRunningPanes
  IF DebugStale THEN EXIT SUB
  StaleVarsTop = VarsTree.TopIndex
  StaleVarsPicked = ""
  IF VarsTree.ItemIndex >= 0 AND VarsTree.ItemIndex <= MAXVARITEMS THEN StaleVarsPicked = VarItemPath(VarsTree.ItemIndex)
  DebugStale = 1
  TreeFilling = 1
  StackTree.Clear
  FrameCount = 0
  StackTree.AddItems TR$("Running…")
  TreeFilling = 0
  FillVariables
  FillWatches
END SUB

SUB RaiseRetryTick
  RaiseTries = RaiseTries + 1
  IF RunState <> "running" OR RaiseTries > 12 THEN
    RaiseRetry.Enabled = 0
  ELSEIF Session.ShowProgram THEN
    RaiseRetry.Enabled = 0
  END IF
END SUB

' The program's windows to the front, its app active (Debug > Show Program
' Window, Continue, its first window, a long step): Studio hands its process
' the front (RPROGRAMSESSION.ShowProgram). Asked (Asked = 1) and it can't —
' no window yet, Wayland —: the status bar says so.
SUB ShowProgram(Asked AS INTEGER)
  IF ProgramIdle THEN EXIT SUB
  IF Session.ShowProgram = 0 AND Asked <> 0 THEN SetState RunState, "The program has no window to bring to the front"
END SUB

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

' (its marker's kind: what the gutter draws — off a ring, never bound to
' code a dashed ring, a log point a diamond, a condition "=", a hit count
' "#": each its own icon)
FUNCTION BpKind(i AS INTEGER) AS STRING
  IF BpOn(i) = 0 THEN
    Result = "breakpoint.disabled"
  ELSEIF BpNever(i) THEN
    Result = "breakpoint.unverified"
  ELSEIF BpLog(i) <> "" THEN
    Result = "breakpoint.log"
  ELSEIF BpCond(i) <> "" THEN
    Result = "breakpoint.conditional"
  ELSEIF BpHit(i) <> "" THEN
    Result = "breakpoint.hitcount"
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
  CodeDoc(d).RemoveMarker BpLine(i), "breakpoint.hitcount"
  CodeDoc(d).RemoveMarker BpLine(i), "breakpoint.unverified"
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
    SetState RunState, TR$("Breakpoint removed: {file}, line {line}", "file", FileTitle(DocFile(d)), "line", LineNo)
  ELSE
    BpPicked = AddBreakpoint(DocFile(d), LineNo)
    SetState RunState, TR$("Breakpoint at {file}, line {line}", "file", FileTitle(DocFile(d)), "line", LineNo)
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
' as debuggers show it; with no code after it in its file, it never
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
  ' (the data tip's tree, its buttons; the arrow carried: Set Next Statement)
  CodeDoc(d).OnDataTipExpand = TipExpand
  CodeDoc(d).OnDataTipAction = TipAction
  CodeDoc(d).OnMarkerDrag = ArrowDragged
  FOR i = 0 TO BpCount - 1
    IF LCASE$(BpFile(i)) = LCASE$(DocFile(d)) THEN CodeDoc(d).AddMarker BpLine(i), BpKind(i)
  NEXT
  CodeDoc(d).DebugHover = Flag(RunState = "paused")
END SUB

SUB CodeGutterClick(LineNo AS INTEGER, Area AS STRING, Sender AS RCodeEditor)
  IF Area = "marker" THEN ToggleBreakpointAt Sender.Tag, LineNo
END SUB

' ---- the Breakpoints pane ------------------------------------------------------------------

FUNCTION BreakpointText(i AS INTEGER) AS STRING
  DIM t AS STRING
  t = FileTitle(BpFile(i)) + ":" + Num(BpLine(i))
  IF BpOn(i) = 0 THEN t = t + "  " + TR$("(off)")
  IF BpNever(i) AND BpOn(i) THEN t = t + "  " + TR$("(no code here: never stops)")
  IF BpCond(i) <> "" THEN t = t + "  " + TR$("when {condition}", "condition", BpCond(i))
  IF BpHit(i) <> "" THEN t = t + "  " + TR$("hit {count}", "count", BpHit(i))
  IF BpLog(i) <> "" THEN t = t + "  " + TR$("log {message}", "message", CHR$(34) + BpLog(i) + CHR$(34))
  Result = t
END FUNCTION

SUB FillBreakpoints
  DIM i AS INTEGER, state AS STRING
  BpTree.Clear
  FOR i = 0 TO BpCount - 1
    BpTree.AddItems BreakpointText(i)
    ' (its state's icon, as the gutter's: the kind after "breakpoint.")
    state = MID$(BpKind(i), 12)
    IF state = "" THEN state = "on"
    BpTree.SetItemValue i, BreakpointText(i), "Breakpoint", "", 0, state
  NEXT
  IF BpCount = 0 THEN BpPicked = -1
  IF BpPicked >= 0 THEN BpTree.ItemIndex = BpPicked
  ShowPickedBreakpoint
  IF BpCount > 0 THEN Dock.PaneTitle("bppanel", TR$("Breakpoints ({n})", "n", BpCount)) ELSE Dock.PaneTitle("bppanel", TR$("Breakpoints"))
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
  SetState RunState, TR$("Breakpoint {text}", "text", BreakpointText(BpPicked))
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
      CodeDoc(d).HideLineValues
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

' The whole program's errors (its main file's analysis, with the files it
' includes), Problems showing them when there are any (S-DESIGN-2).
FUNCTION ProgramErrors AS INTEGER
  DIM d AS INTEGER
  ' (every open file as its editor has it — a module the assistant's change
  ' was just applied to, not saved yet, counts as its editor shows it; it
  ' counted as the disk had it, and "1 error" was said of a program that
  ' has none); the editors' popups go: the program comes up over them
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) THEN
      IF IsBasicFile(DocFile(d)) THEN Lang.Update DocFile(d), CodeDoc(d).Text
      CodeDoc(d).HidePopups
    END IF
  NEXT
  FillProblemsOf ProjectMain
  Result = Lang.ErrorCount
  ' (no errors: Problems back to the file shown)
  IF Result = 0 THEN FillProblems ActiveCode
END FUNCTION

' (the project's main file isn't there: none set, gone from the disk — on
' the web a file of the site's isn't in the page's store, so its editor or
' a read says whether it is there)
FUNCTION MainMissing AS INTEGER
  DIM t AS STRING
  IF ProjectMain = "" THEN
    Result = 1
  ELSEIF FILEEXISTS(ProjectMain) = 0 AND DocOf(ProjectMain) < 0 THEN
    t = Proj.FileText(ProjectMain)
    Result = Flag(Proj.Error <> "")
  END IF
END FUNCTION

SUB StartProgram(WithDebugger AS INTEGER)
  ' (Run always says why it didn't start: Robert 2026-10-28, a Run that
  ' did nothing and said nothing)
  IF ProjectOpen = 0 THEN
    OutputLine TR$("Nothing to run: open a project or a program first (File > Open, or an example).")
    SetState "error", "Nothing to run: no project is open"
    EXIT SUB
  END IF
  IF RunState = "running" OR RunState = "paused" THEN
    IF RunState = "running" THEN
      SetState RunState, TR$("{file} is running already: Stop it first (Shift+F5)", "file", FileTitle(ProjectMain))
    ELSE
      SetState RunState, TR$("{file} is paused already: Stop it first (Shift+F5)", "file", FileTitle(ProjectMain))
    END IF
    EXIT SUB
  END IF
  IF MainMissing THEN
    IF ProjectMain = "" THEN
      OutputLine TR$("Can't run: the project's main file isn't set. Choose it in the project's properties: select the project's name in the Project pane, then MainFile.")
    ELSE
      OutputLine TR$("Can't run: the project's main file {file} isn't there. Choose it in the project's properties: select the project's name in the Project pane, then MainFile.", "file", ProjectMain)
    END IF
    SetState "error", "Can't run: no main file"
    OutputBox.Page = "output"
    Dock.ShowPane("outputbox")
    EXIT SUB
  END IF
  RunOutputSeen = 0
  ' (F5 to the first form: docs/studio-wow.md PERF-8)
  RunStarted = TIMER
  PerfForm = -1
  ' (VB's way: what is being run is saved first — not run when a save was
  ' declined: its file changed outside Studio)
  IF SaveAllOk = 0 THEN EXIT SUB
  AnalyzeActive
  ' (the program is its main file and what that includes: their errors
  ' stop it, whichever file is shown — Problems lists them: S-DESIGN-2)
  IF ProgramErrors > 0 THEN
    SetState "error", TRN$("Can't run: {n} error. The first is selected.", "Can't run: {n} errors. The first is selected.", Lang.ErrorCount)
    OutputBox.Page = "problems"
    Dock.ShowPane("outputbox")
    GoToFirstProblem
    EXIT SUB
  END IF
  SyncAllBreakpoints
  ResetNeverBreakpoints
  RunRaised = 0
  StopLog = ""
  OutputBox.Clear "output"
  ' (the assistant sees the end of this run's output, not the last one's)
  AITools.RunOutput = ""
  OutputBox.Page = "output"
  Dock.ShowPane("outputbox")
  ClearDebugPanes
  Session.Program = ProjectMain
  Session.Debug = WithDebugger
  Session.BreakOnError = Flag(WithDebugger <> 0 AND BreakOnErrors <> 0)
  ' (a crash report RapidR may write while it runs: crash.inc)
  CrashRunStarts
  IF Session.Start THEN
    SetState "running", TR$("Running {file}", "file", FileTitle(ProjectMain))
    ' (with the debugger: the debugging layout — S9)
    IF WithDebugger THEN DebugLayoutRunStarts
  ELSE
    OutputLine TR$("Can't run {file}: {error}", "file", ProjectMain, "error", Session.Error)
    SetState "error", TR$("Can't run {file}", "file", FileTitle(ProjectMain))
  END IF
END SUB

' (Run with errors: the editor at the first one — docs/studio-wow.md RUN-3)
SUB GoToFirstProblem
  DIM d AS INTEGER, text AS STRING, row AS STRING, i AS INTEGER, n AS INTEGER
  d = ActiveCode
  IF d < 0 THEN EXIT SUB
  ' (the project's: its errors come first, whichever file has them)
  IF InProject(DocFile(d)) THEN text = ProblemsText(ProjectMain) ELSE text = Lang.Diagnostics(DocFile(d))
  n = TALLY(text, CHR$(10))
  FOR i = 1 TO n
    row = FIELD$(text, CHR$(10), i)
    IF FIELD$(row, CHR$(9), 1) = "error" THEN
      ShowLocation FIELD$(row, CHR$(9), 5), VAL(FIELD$(row, CHR$(9), 2)), ""
      IF FIELD$(row, CHR$(9), 5) = "" THEN
        Dock.FocusPane(DocPane(d))
        CodeDoc(d).GotoLine VAL(FIELD$(row, CHR$(9), 2)) - 1
        EditorFacts d
      END IF
      EXIT SUB
    END IF
  NEXT
END SUB

' ---- the editing layout and the debugging layout (S9) ----------------------------------
' While you edit, the bottom panel (Output with its Output, Build and Problems
' pages, Immediate, Breakpoints) spans the whole width under the documents.
' When a run with the debugger starts, Studio switches to the debugging
' layout: Variables, Watch, Call Stack and Breakpoints in a group beside
' Output and Immediate; at a pause Variables comes to the front. When the
' program ends it switches back. Each layout keeps its own sizes (the bottom
' panel's height; the debug panes' width), saved with the settings.
' Settings > General > Debug panes: Automatic (the default), Always (the
' debugging layout all the time) or Never (only when asked). View > Debug
' Panes (Ctrl+Shift+D) shows or hides them now: shown by hand they stay
' (pinned) after the program ends until hidden again. The switch is said in
' the status bar (a screen reader hears it); it never takes the keyboard.

' (DebugPanesMode, decl.inc: Automatic "auto", Always "always", Never
' "never". The debugging layout is shown; turned on by hand: kept after
' the run; turned off by hand during a run: not brought back until the
' next one. EditBottom, DebugBottom, DebugWidth, decl.inc: each layout's
' sizes — the bottom panel's height while editing and while debugging,
' the debug panes' width; 0: as the dock has it)
DIM DebugLayoutOn AS INTEGER
DIM DebugPanesPinned AS INTEGER
DIM DebugPanesDeclined AS INTEGER
' (the status bar's note of the switch, then the project's name again)
DIM LayoutNoteTimer AS RTimer
LayoutNoteTimer.Interval = 4000
LayoutNoteTimer.Enabled = 0
LayoutNoteTimer.OnTimer = LayoutNoteGone

' (a sentence in the status bar's middle panel for a few seconds: the
' status bar is a live region, a screen reader reads it out)
SUB LayoutNote(Text AS STRING)
  Status.Panel(1).Caption = Text
  LayoutNoteTimer.Enabled = 0
  LayoutNoteTimer.Enabled = 1
END SUB

SUB LayoutNoteGone
  LayoutNoteTimer.Enabled = 0
  LayOutStatus
END SUB

' (the sizes the layout shown has now, kept for the next time it shows)
SUB RememberLayoutSizes
  DIM h AS INTEGER, w AS INTEGER
  h = Dock.PaneExtent("outputbox", "height")
  IF DebugLayoutOn THEN
    IF h > 0 THEN DebugBottom = h
    w = Dock.PaneExtent("varstree", "width")
    IF w > 0 THEN DebugWidth = w
  ELSE
    IF h > 0 THEN EditBottom = h
  END IF
END SUB

' The debugging layout: the debug panes beside Output, Breakpoints with
' them, at their sizes. Remember: the editing layout's sizes kept first.
SUB DebugPanesIn(Remember AS INTEGER)
  DIM w AS INTEGER, total AS INTEGER
  IF Remember THEN RememberLayoutSizes
  IF Dock.PaneState("varstree") = "hidden" THEN Dock.DockPane("varstree", "right:outputbox")
  IF Dock.PaneState("watchpanel") = "hidden" THEN Dock.DockPane("watchpanel", "tab:varstree")
  IF Dock.PaneState("stacktree") = "hidden" THEN Dock.DockPane("stacktree", "tab:varstree")
  Dock.DockPane("bppanel", "tab:varstree")
  Dock.ShowPane("varstree")
  IF DebugBottom > 0 THEN Dock.PaneExtent("outputbox", "height", DebugBottom)
  ' (the first time: the debug panes three fifths of the bottom panel —
  ' their four tabs have room)
  w = DebugWidth
  IF w <= 0 THEN
    total = Dock.PaneExtent("outputbox", "width") + Dock.PaneExtent("varstree", "width")
    IF total >= 600 THEN w = total * 3 \ 5
  END IF
  IF w > 0 THEN Dock.PaneExtent("varstree", "width", w)
  DebugLayoutOn = 1
  CheckCmd "view.debugPanes", 1
END SUB

' The editing layout: the debug panes gone, Breakpoints a tab beside
' Output, the bottom panel the whole width, at its height.
SUB DebugPanesOut(Remember AS INTEGER)
  DIM a AS STRING, imm AS INTEGER
  IF Remember THEN RememberLayoutSizes
  ' (the keyboard in a pane that goes: to the code, never lost)
  a = LCASE$(Dock.ActivePane)
  imm = Dock.PaneVisible("immediatebox")
  Dock.DockPane("bppanel", "tab:outputbox")
  ' (Output — or Immediate, if it was — stays in front: Breakpoints joins
  ' them as a tab behind)
  IF imm THEN Dock.ShowPane("immediatebox") ELSE Dock.ShowPane("outputbox")
  Dock.HidePane("varstree")
  Dock.HidePane("watchpanel")
  Dock.HidePane("stacktree")
  IF EditBottom > 0 THEN Dock.PaneExtent("outputbox", "height", EditBottom)
  DebugLayoutOn = 0
  CheckCmd "view.debugPanes", 0
  IF a = "varstree" OR a = "watchpanel" OR a = "stacktree" THEN
    IF ActiveCode >= 0 THEN Dock.FocusPane(DocPane(ActiveCode))
  END IF
END SUB

' (the switch, said)
SUB ShowDebugLayout
  IF DebugLayoutOn THEN EXIT SUB
  DebugPanesIn 1
  LayoutNote TR$("Debugging layout: Variables, Watch, Call Stack and Breakpoints beside Output")
END SUB

SUB ShowEditingLayout
  IF DebugLayoutOn = 0 THEN EXIT SUB
  DebugPanesOut 1
  LayoutNote TR$("Editing layout: the bottom panel is full width")
END SUB

' A run with the debugger starts: the debugging layout (Automatic, Always).
SUB DebugLayoutRunStarts
  DebugPanesDeclined = 0
  IF DebugPanesMode <> "never" THEN ShowDebugLayout
END SUB

' A pause: the debugging layout (unless hidden by hand during this run, or
' Never), Variables in front.
SUB DebugLayoutPaused
  IF DebugPanesMode <> "never" AND DebugPanesDeclined = 0 THEN ShowDebugLayout
  IF DebugLayoutOn THEN Dock.ShowPane("varstree")
END SUB

' The program ended: the editing layout, unless pinned or Always.
SUB DebugLayoutRunEnds
  DebugPanesDeclined = 0
  IF DebugPanesMode <> "always" AND DebugPanesPinned = 0 THEN ShowEditingLayout
END SUB

' View > Debug Panes (Ctrl+Shift+D): shown or hidden now; shown by hand,
' they stay after the program ends (pinned) until hidden again.
SUB ToggleDebugPanes
  IF DebugLayoutOn THEN
    DebugPanesPinned = 0
    IF RunState = "running" OR RunState = "paused" THEN DebugPanesDeclined = 1
    ShowEditingLayout
  ELSE
    DebugPanesPinned = 1
    DebugPanesDeclined = 0
    ShowDebugLayout
  END IF
END SUB

' (a debug pane asked for — View > Variables, Add Watch —: the debugging
' layout shown, as by hand)
SUB NeedDebugPanes
  IF DebugLayoutOn THEN EXIT SUB
  DebugPanesPinned = 1
  DebugPanesDeclined = 0
  ShowDebugLayout
END SUB

' A layout loaded (at start-up, a project's session, Reset Layout): the
' layout the mode says, at its sizes — whatever the text had (a layout
' saved while a program was paused).
SUB DebugLayoutLoaded
  IF DebugPanesMode = "always" OR DebugPanesPinned THEN
    DebugPanesOut 0
    DebugPanesIn 0
  ELSE
    DebugPanesIn 0
    DebugPanesOut 0
  END IF
END SUB

' Settings > General > Debug panes chosen.
SUB SetDebugPanesMode(Mode AS STRING)
  DebugPanesMode = Mode
  IF Mode = "always" THEN ShowDebugLayout
  IF Mode = "never" AND DebugPanesPinned = 0 THEN ShowEditingLayout
  IF Mode = "auto" AND DebugPanesPinned = 0 AND RunState <> "running" AND RunState <> "paused" THEN ShowEditingLayout
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
  StepRaise.Enabled = 0
  IF RunState = "running" OR RunState = "paused" THEN Session.Stop
END SUB

' Run > Run in Browser: the program's web build served on this machine and
' opened in the default browser (the desktop's; on the web Studio is in
' the browser: Run runs it here).
SUB RunInBrowser
  DIM why AS STRING
  IF ProjectOpen = 0 THEN EXIT SUB
  IF SaveAllOk = 0 THEN EXIT SUB
  AnalyzeActive
  ' (the program is its main file and what that includes: their errors
  ' stop it, whichever file is shown — Problems lists them: S-DESIGN-2)
  IF ProgramErrors > 0 THEN
    SetState "error", TRN$("Can't run: {n} error. The first is selected.", "Can't run: {n} errors. The first is selected.", Lang.ErrorCount)
    GoToFirstProblem
    EXIT SUB
  END IF
  Session.Program = ProjectMain
  Dock.ShowPane("outputbox")
  OutputLine TR$("Run in Browser: building {file} for the web...", "file", FileTitle(ProjectMain))
  Session.RunInBrowser
  why = Session.Error
  IF why = "" THEN
    SetState RunState, TR$("Opening {file} in the browser...", "file", FileTitle(ProjectMain))
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
  IF ProgramIdle THEN
    StartProgram 1
  ELSE
    ' (the program's turn until it gets there)
    ShowProgram 0
  END IF
END SUB

' (the program's output as it comes; OutLineOpen: its last line isn't ended)
DIM OutLineOpen AS INTEGER

SUB ProgramOutput(Text AS STRING)
  IF Text <> "" THEN RunOutputSeen = 1
  OutputBox.Write Text
  AITools.AddRunOutput Text
  ' (AI apps' read_output: mcp.inc)
  McpProgramOutput Text
  IF Text <> "" THEN OutLineOpen = Flag(RIGHT$(Text, 1) <> CHR$(10))
END SUB

' (a line of Studio's own after the program's: on a line of its own)
SUB OutputAfterProgram(Text AS STRING)
  IF OutLineOpen THEN OutputBox.Write CHR$(10)
  OutLineOpen = 0
  OutputLine Text
END SUB

SUB ProgramStopped(Reason AS STRING, InFile AS STRING, LineNo AS INTEGER)
  DIM what AS STRING, kind AS STRING, at AS STRING
  StepRaise.Enabled = 0
  StaleTimer.Enabled = 0
  ' (the run's stops, for the flows' stops:report)
  StopLog = StopLog + " " + Num(LineNo) + ":" + Reason
  PausedFile = DebugPath(InFile)
  PausedLine = LineNo
  at = FileTitle(PausedFile) + ":" + Num(LineNo)
  SELECT CASE Reason
    CASE "breakpoint": what = TR$("Breakpoint: {place}", "place", at)
    CASE "entry": what = TR$("Paused at the start: {place}", "place", at)
    CASE "exception": what = TR$("Run-time error: {place}", "place", at)
    CASE "goto": what = TR$("Next statement: {place}", "place", at)
    CASE ELSE
      what = TR$("Paused: {place}", "place", at)
      ' (while the program waits for its events: a ShowModal, a dialog)
      IF Reason = "pause" AND Session.StopMessage <> "" THEN what = TR$("Paused, {message}: {place}", "message", LCASE$(Session.StopMessage), "place", at)
  END SELECT
  IF Reason = "exception" THEN kind = "exception" ELSE kind = "current"
  SetState "paused", what
  ' (the data tips' values are the last stop's: they compare with this one's)
  TipNewStop
  ClearPausedMarks
  ShowLocation PausedFile, LineNo, kind
  IF Reason = "exception" THEN
    OutputAfterProgram FileTitle(PausedFile) + ":" + Num(LineNo) + ": " + Session.StopMessage
  END IF
  ' (a new stop: its references are new; Locals and the globals are in)
  RefsIn = "|" + Num(Session.LocalsRef) + "|" + Num(Session.GlobalsRef) + "|" + Num(Session.ComponentsRef) + "|"
  FillCallStack
  FillVariables
  FillWatches
  DebugStale = 0
  ShowPausedValues
  ' (the debugging layout, Variables in front — S9)
  DebugLayoutPaused
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
  ' (its first window comes up in front of Studio: the system leaves a
  ' window a background process opens behind the active app's)
  IF RunRaised = 0 AND RunState = "running" THEN
    RunRaised = 1
    IF Session.ShowProgram = 0 THEN
      RaiseTries = 0
      RaiseRetry.Enabled = 1
    END IF
  END IF
END SUB

' The values at the ends of the paused routine's lines (the code's helpers,
' docs/code-helpers-plan.md H4): its locals and the globals, on the lines
' from the routine's first to the paused one — the program's own lines: the
' 40 before the pause.
SUB ShowPausedValues
  DIM d AS INTEGER, all AS STRING, outl AS STRING, n AS INTEGER, k AS INTEGER, e AS STRING, kd AS STRING, first AS INTEGER
  d = DocOf(PausedFile)
  IF d < 0 OR PausedLine < 1 THEN EXIT SUB
  IF Session.LocalsRef > 0 THEN all = PausedRows(Session.LocalsRef)
  all = all + PausedRows(Session.GlobalsRef)
  first = PausedLine - 40
  IF first < 1 THEN first = 1
  IF FrameCount > 0 THEN
    IF FrameName(0) <> "(the program)" THEN
      outl = CodeDoc(d).Outline
      n = TALLY(outl, CHR$(10)) + 1
      FOR k = 1 TO n
        e = FIELD$(outl, CHR$(10), k)
        kd = LCASE$(FIELD$(e, CHR$(9), 1))
        IF (kd = "sub" OR kd = "function") AND LCASE$(FIELD$(e, CHR$(9), 2)) = LCASE$(FrameName(0)) THEN first = VAL(FIELD$(e, CHR$(9), 3))
      NEXT
    END IF
  END IF
  CodeDoc(d).ShowLineValues all, first, PausedLine
END SUB

' (reference Ref's values as ShowLineValues takes them: name, value, kind;
' an $INCLUDE's names and the components left out)
FUNCTION PausedRows(Ref AS INTEGER) AS STRING
  DIM rows AS STRING, n AS INTEGER, k AS INTEGER, row AS STRING, kd AS STRING
  IF Ref <= 0 THEN EXIT FUNCTION
  rows = Session.Variables(Ref)
  n = TALLY(rows, CHR$(10))
  FOR k = 1 TO n
    row = FIELD$(rows, CHR$(10), k)
    kd = FIELD$(row, CHR$(9), 3)
    IF kd <> "Include" AND kd <> "Components" THEN
      Result = Result + FIELD$(row, CHR$(9), 1) + CHR$(9) + FIELD$(row, CHR$(9), 2) + CHR$(9) + kd + CHR$(10)
    END IF
  NEXT
END FUNCTION

SUB ProgramContinued
  ForgetSetValues
  SetState "running", TR$("Running {file}", "file", FileTitle(ProjectMain))
  ClearPausedMarks
  ' (still running in a moment: the panes say so — ShowRunningPanes)
  StaleTimer.Enabled = 0
  StaleTimer.Enabled = 1
END SUB

SUB ProgramExited(Code AS INTEGER)
  StepRaise.Enabled = 0
  StaleTimer.Enabled = 0
  DebugStale = 0
  OutputAfterProgram TR$("[{file} ended, exit code {code}]", "file", FileTitle(ProjectMain), "code", Code)
  ' (it ended at once, showed no window and printed nothing: maybe it isn't
  ' the program's start — a project whose main file became a module)
  IF Code = 0 AND PerfForm < 0 AND RunOutputSeen = 0 AND TIMER - RunStarted < 3 THEN
    OutputLine TR$("{file} ran and ended at once: it showed no window and printed nothing. If the program starts in another file, make that the main file: select the project's name in the Project pane, then MainFile in Properties.", "file", FileTitle(ProjectMain))
  END IF
  IF Code = 0 THEN SetState "stopped", "Ready" ELSE SetState "error", TR$("Ended with exit code {code}", "code", Code)
  ' (RapidR stopped it — a crash in RapidR's own code: said plainly, with
  ' its report — crash.inc)
  CrashProgramEnded Code
  ClearPausedMarks
  ClearDebugPanes
  ' (the editing layout again — S9)
  DebugLayoutRunEnds
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

' (item i a value: drawn as one — its kind's icon, its value styled by kind
' — and read as a sentence; Hint is what the registry knows of a property:
' color, handle, const:alClient, readonly. A row with no kind stays text.)
SUB TreeSetValue(w AS INTEGER, i AS INTEGER, Nm AS STRING, Kind AS STRING, Value AS STRING, Count AS INTEGER, Hint AS STRING)
  IF i >= 0 AND i <= MAXVARITEMS THEN
    IF w = 0 THEN
      VarItemHint(i) = Hint: VarItemValue(i) = Value
    ELSE
      WatchItemHint(i) = Hint: WatchItemValue(i) = Value
    END IF
  END IF
  IF Kind = "" THEN EXIT SUB
  IF w = 0 THEN VarsTree.SetItemValue i, Nm, Kind, Value, Count, Hint ELSE WatchTree.SetItemValue i, Nm, Kind, Value, Count, Hint
END SUB

FUNCTION TreeCount(w AS INTEGER) AS INTEGER
  IF w = 0 THEN Result = VarsTree.ItemCount ELSE Result = WatchTree.ItemCount
END FUNCTION

SUB TreeItemData(w AS INTEGER, i AS INTEGER, Ref AS INTEGER, Path AS STRING)
  IF i > MAXVARITEMS THEN EXIT SUB
  IF w = 0 THEN
    VarItemRef(i) = Ref: VarItemPath(i) = Path
    VarItemExpr(i) = "": VarItemKind(i) = "": VarItemMore(i) = -1
    VarItemHint(i) = "": VarItemValue(i) = ""
    IF Ref > 0 THEN VarsTree.Item(i).HasChildren = 1
  ELSE
    WatchItemRef(i) = Ref: WatchItemPath(i) = Path
    WatchItemExpr(i) = "": WatchItemKind(i) = "": WatchItemMore(i) = -1
    WatchItemHint(i) = "": WatchItemValue(i) = ""
    IF Ref > 0 THEN WatchTree.Item(i).HasChildren = 1
  END IF
END SUB

' (an item's value: the expression naming it, its type; changed since the
' last stop: highlighted and marked)
SUB TreeItemValue(w AS INTEGER, i AS INTEGER, Expr AS STRING, Kind AS STRING, Changed AS INTEGER)
  IF i > MAXVARITEMS THEN EXIT SUB
  IF w = 0 THEN
    VarItemExpr(i) = Expr: VarItemKind(i) = Kind
    IF Changed THEN VarsTree.Item(i).Changed = 1
  ELSE
    WatchItemExpr(i) = Expr: WatchItemKind(i) = Kind
    IF Changed THEN WatchTree.Item(i).Changed = 1
  END IF
END SUB

' (references a tree needs that this stop hasn't fetched: asked for once
' the tree is filled — OnVariables fills it again)
DIM TreeFetch AS STRING

SUB FetchTreeRefs
  DIM want AS STRING, n AS INTEGER, k AS INTEGER, r AS INTEGER
  want = TreeFetch
  TreeFetch = ""
  n = TALLY(want, "|")
  FOR k = 2 TO n + 1
    r = VAL(FIELD$(want, "|", k))
    IF r > 0 AND RefIn(r) = 0 THEN Session.Expand r
  NEXT
END SUB

' The children of reference Ref under item Parent of tree w, depth first (a
' child is added after its parent's whole subtree: depth first keeps that
' the end of the list); an open child's children too — fetched once the
' tree is filled when this stop hasn't yet. An RDataFrame's rows come under
' its columns' names (Columns); a big array, frame or RNum shows a page,
' then "… more" (Total: how many there are).
SUB AddValueChildren(w AS INTEGER, Parent AS INTEGER, Ref AS INTEGER, Path AS STRING, Depth AS INTEGER, Columns AS STRING, Total AS INTEGER)
  DIM rows AS STRING, n AS INTEGER, k AS INTEGER, row AS STRING, nm AS STRING, cref AS INTEGER, cpath AS STRING, item AS INTEGER, opened AS STRING
  IF Depth > 12 THEN EXIT SUB
  rows = Session.Variables(Ref)
  n = TALLY(rows, CHR$(10))
  IF w = 0 THEN opened = VarsOpen ELSE opened = WatchOpen
  IF Columns <> "" THEN
    TreeAddChild w, Parent, "[" + REPLACESUBSTR$(Columns, CHR$(31), "  |  ") + "]"
    TreeItemData w, TreeCount(w) - 1, 0, Path + "/[columns]"
  END IF
  FOR k = 1 TO n
    IF TreeCount(w) >= MAXVARITEMS THEN EXIT SUB
    row = FIELD$(rows, CHR$(10), k)
    nm = FIELD$(row, CHR$(9), 1)
    cref = VAL(FIELD$(row, CHR$(9), 4))
    cpath = Path + "/" + nm
    ' (the globals an $INCLUDEd file made — RAPIDQ.INC's constants, when a
    ' program includes it —, under one closed item after the program's own:
    ' "$INCLUDE RAPIDQ.INC (n names)"; a .rr program's built-in constants
    ' are no globals, never listed)
    IF FIELD$(row, CHR$(9), 3) = "Include" THEN
      TreeAddChild w, Parent, nm + " " + FIELD$(row, CHR$(9), 2)
    ELSE
      TreeAddChild w, Parent, ValueText(nm, FIELD$(row, CHR$(9), 2))
    END IF
    item = TreeCount(w) - 1
    TreeItemData w, item, cref, cpath
    TreeSetValue w, item, nm, FIELD$(row, CHR$(9), 3), FIELD$(row, CHR$(9), 2), VAL(FIELD$(row, CHR$(9), 5)), FIELD$(row, CHR$(9), 10)
    TreeItemValue w, item, FIELD$(row, CHR$(9), 6), FIELD$(row, CHR$(9), 3), VAL(FIELD$(row, CHR$(9), 7))
    IF cref > 0 AND IsOpen(opened, cpath) THEN
      IF RefIn(cref) THEN
        AddValueChildren w, item, cref, cpath, Depth + 1, FIELD$(row, CHR$(9), 8), VAL(FIELD$(row, CHR$(9), 5))
      ELSE
        TreeFetch = TreeFetch + "|" + Num(cref)
      END IF
    END IF
  NEXT
  ' (more than came: the next page when it's opened)
  IF Total > n AND n > 0 AND TreeCount(w) < MAXVARITEMS THEN
    TreeAddChild w, Parent, TR$("… {n} more", "n", Total - n)
    item = TreeCount(w) - 1
    TreeItemData w, item, 0, Path + "/[more]"
    TreeSetValue w, item, TR$("… {n} more", "n", Total - n), "More", "", 0, ""
    IF item <= MAXVARITEMS THEN
      IF w = 0 THEN
        VarItemMore(item) = n: VarItemRef(item) = Ref: VarsTree.Item(item).HasChildren = 1
      ELSE
        WatchItemMore(item) = n: WatchItemRef(item) = Ref: WatchTree.Item(item).HasChildren = 1
      END IF
    END IF
  END IF
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
  DIM top AS INTEGER, root AS INTEGER, picked AS STRING
  TreeFilling = 1
  top = VarsTree.TopIndex
  IF VarsTree.ItemIndex >= 0 AND VarsTree.ItemIndex <= MAXVARITEMS THEN picked = VarItemPath(VarsTree.ItemIndex)
  ' (after a run on: where it was at the last pause — ShowRunningPanes)
  IF DebugStale THEN
    top = StaleVarsTop
    picked = StaleVarsPicked
  END IF
  VarsTree.Clear
  IF RunState <> "paused" THEN
    IF RunState = "running" THEN
      VarsTree.AddItems TR$("Running… the values show when it pauses (F9 sets a breakpoint).")
    ELSE
      VarsTree.AddItems TR$("Not paused: F9 sets a breakpoint, F5 runs.")
    END IF
    TreeItemData 0, 0, 0, ""
    TreeFilling = 0
    EXIT SUB
  END IF
  VarsTree.AddItems TR$("Locals")
  VarsTree.SetItemValue 0, TR$("Locals"), "Locals", ""
  TreeItemData 0, 0, 0, "L"
  root = 0
  IF Session.LocalsRef > 0 THEN AddValueChildren 0, root, Session.LocalsRef, "L", 0, "", 0
  IF VarsTree.ItemCount = 1 THEN
    VarsTree.AddChildItems 0, TR$("(none)")
    TreeItemData 0, 1, 0, "L/"
  END IF
  VarsTree.AddItems TR$("Globals")
  root = VarsTree.ItemCount - 1
  VarsTree.SetItemValue root, TR$("Globals"), "Globals", ""
  TreeItemData 0, root, 0, "G"
  AddValueChildren 0, root, Session.GlobalsRef, "G", 0, "", 0
  ' (the program's components — its forms, frames, timers —, each opening
  ' to its properties and the components it holds; filled once opened)
  VarsTree.AddItems TR$("Components")
  root = VarsTree.ItemCount - 1
  VarsTree.SetItemValue root, TR$("Components"), "Components", ""
  TreeItemData 0, root, Session.ComponentsRef, "C"
  IF IsOpen(VarsOpen, "C") THEN AddValueChildren 0, root, Session.ComponentsRef, "C", 0, "", 0
  ReopenTree 0
  IF top < VarsTree.ItemCount THEN VarsTree.TopIndex = top
  ' (the item picked stays picked)
  IF picked <> "" THEN SelectPath 0, picked
  TreeFilling = 0
  FetchTreeRefs
END SUB

' (tree w's item at that path selected, when it's there)
SUB SelectPath(w AS INTEGER, Path AS STRING)
  DIM k AS INTEGER
  FOR k = 0 TO TreeCount(w) - 1
    IF k > MAXVARITEMS THEN EXIT SUB
    IF w = 0 THEN
      IF VarItemPath(k) = Path THEN VarsTree.ItemIndex = k: EXIT SUB
    ELSE
      IF WatchItemPath(k) = Path THEN WatchTree.ItemIndex = k: EXIT SUB
    END IF
  NEXT
END SUB

SUB FillWatches
  DIM rows AS STRING, n AS INTEGER, k AS INTEGER, row AS STRING, expr AS STRING, value AS STRING, wref AS INTEGER, item AS INTEGER, picked AS STRING
  TreeFilling = 1
  IF WatchTree.ItemIndex >= 0 AND WatchTree.ItemIndex <= MAXVARITEMS THEN picked = WatchItemPath(WatchTree.ItemIndex)
  WatchTree.Clear
  rows = Session.WatchValues
  n = TALLY(rows, CHR$(10))
  FOR k = 1 TO n
    row = FIELD$(rows, CHR$(10), k)
    expr = FIELD$(row, CHR$(9), 1)
    value = FIELD$(row, CHR$(9), 2)
    wref = VAL(FIELD$(row, CHR$(9), 4))
    IF RunState = "running" THEN
      value = TR$("(running)")
    ELSEIF RunState <> "paused" THEN
      value = TR$("(not paused)")
    END IF
    WatchTree.AddItems ValueText(expr, value)
    item = WatchTree.ItemCount - 1
    TreeItemData 1, item, wref, "W" + Num(k)
    IF RunState = "paused" THEN TreeSetValue 1, item, expr, FIELD$(row, CHR$(9), 3), value, VAL(FIELD$(row, CHR$(9), 6)), ""
    TreeItemValue 1, item, expr, FIELD$(row, CHR$(9), 3), Flag(RunState = "paused" AND VAL(FIELD$(row, CHR$(9), 5)) <> 0)
    IF RunState = "paused" AND wref > 0 AND IsOpen(WatchOpen, "W" + Num(k)) THEN
      IF RefIn(wref) THEN AddValueChildren 1, item, wref, "W" + Num(k), 0, "", VAL(FIELD$(row, CHR$(9), 6)) ELSE TreeFetch = TreeFetch + "|" + Num(wref)
    END IF
  NEXT
  ReopenTree 1
  IF picked <> "" THEN SelectPath 1, picked
  TreeFilling = 0
  FetchTreeRefs
END SUB

' A value opened by the user: its children fetched (then filled in:
' OnVariables); "… more" opened: the next page.
SUB VarsExpanding(Index AS INTEGER, AllowExpansion AS INTEGER)
  IF TreeFilling OR Index < 0 OR Index > MAXVARITEMS THEN EXIT SUB
  IF VarItemMore(Index) >= 0 THEN
    AllowExpansion = 0
    Session.ExpandMore VarItemRef(Index), VarItemMore(Index)
    EXIT SUB
  END IF
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
  IF WatchItemMore(Index) >= 0 THEN
    AllowExpansion = 0
    Session.ExpandMore WatchItemRef(Index), WatchItemMore(Index)
    EXIT SUB
  END IF
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
    RefsIn = "|" + Num(Session.LocalsRef) + "|" + Num(Session.GlobalsRef) + "|" + Num(Session.ComponentsRef) + "|"
  ELSEIF RefIn(Ref) = 0 THEN
    RefsIn = RefsIn + Num(Ref) + "|"
  END IF
  FillVariables
  FillWatches
  IF TipShown AND Ref <> 0 THEN ShowTip
END SUB

' ---- values set in place ------------------------------------------------------------------------
' F2 (or a click on the selected item) on a value in Variables or Watch
' edits it, as debuggers' variables do: what's typed — an
' expression — is set in the paused program (SetVariable: the frame's
' local, a global, an array's element, a TYPE's field, a component's
' property), then every value is fetched again. The item reads
' "name = value": what follows " = " is the new value (all of it, when the
' " = " went). A watch's own line edits its expression (paused or not).

' (the expression an item stands for: its path's names joined, an array's
' element "(3)" on its array, a field or a property after a dot; Base the
' watch's expression)
FUNCTION PathExpr(Path AS STRING, Base AS STRING) AS STRING
  DIM n AS INTEGER, k AS INTEGER, part AS STRING, e AS STRING
  n = TALLY(Path, "/")
  e = Base
  FOR k = 2 TO n + 1
    part = FIELD$(Path, "/", k)
    IF e = "" THEN
      e = part
    ELSEIF LEFT$(part, 1) = "(" THEN
      e = e + part
    ELSE
      e = e + "." + part
    END IF
  NEXT
  Result = e
END FUNCTION

' (an item's name: its path's last part)
FUNCTION PathName(Path AS STRING) AS STRING
  Result = MID$(Path, RINSTR(Path, "/") + 1)
END FUNCTION

' (what was typed for the value: after "name = ", or the whole text)
FUNCTION TypedValue(Text AS STRING) AS STRING
  DIM k AS INTEGER
  k = INSTR(Text, " = ")
  IF k > 0 THEN Result = MID$(Text, k + 3) ELSE Result = Text
  Result = LTRIM$(RTRIM$(Result))
END FUNCTION

' (a watch's expression a value's place can be named from: a name, its
' fields and elements — not "a + b")
FUNCTION PlainWatch(Expr AS STRING) AS INTEGER
  DIM i AS INTEGER, c AS STRING
  Result = Flag(Expr <> "")
  FOR i = 1 TO LEN(Expr)
    c = MID$(Expr, i, 1)
    IF INSTR("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_.()$%&!#,", c) = 0 THEN Result = 0
  NEXT
END FUNCTION

' (watch k's expression, from 1)
FUNCTION WatchExpr(k AS INTEGER) AS STRING
  Result = FIELD$(Session.Watches, CHR$(10), k)
END FUNCTION

SUB SetValue(Expr AS STRING, Value AS STRING, Shown AS STRING)
  IF Expr = "" OR Value = "" OR RunState <> "paused" THEN EXIT SUB
  SetVarExpr = Expr
  SetVarOld = EditOld
  SetVarUndo = 0
  SetVarImm = 0
  SetVarShown = Shown
  SetVarId = Session.SetVariable(Expr, Value)
END SUB

' (tree w's item i: its expression, its kind, its hint, its name, its value)
FUNCTION ItemExpr(w AS INTEGER, i AS INTEGER) AS STRING
  IF i < 0 OR i > MAXVARITEMS THEN EXIT FUNCTION
  IF w = 0 THEN Result = VarItemExpr(i) ELSE Result = WatchItemExpr(i)
END FUNCTION

FUNCTION ItemKind(w AS INTEGER, i AS INTEGER) AS STRING
  IF i < 0 OR i > MAXVARITEMS THEN EXIT FUNCTION
  IF w = 0 THEN Result = VarItemKind(i) ELSE Result = WatchItemKind(i)
END FUNCTION

FUNCTION ItemHint(w AS INTEGER, i AS INTEGER) AS STRING
  IF i < 0 OR i > MAXVARITEMS THEN EXIT FUNCTION
  IF w = 0 THEN Result = VarItemHint(i) ELSE Result = WatchItemHint(i)
END FUNCTION

FUNCTION ItemValue(w AS INTEGER, i AS INTEGER) AS STRING
  IF i < 0 OR i > MAXVARITEMS THEN EXIT FUNCTION
  IF w = 0 THEN Result = VarItemValue(i) ELSE Result = WatchItemValue(i)
END FUNCTION

FUNCTION ItemName(w AS INTEGER, i AS INTEGER) AS STRING
  DIM p AS STRING
  IF i < 0 OR i > MAXVARITEMS THEN EXIT FUNCTION
  IF w = 0 THEN p = VarItemPath(i) ELSE p = WatchItemPath(i)
  ' (a watch's own line: its expression)
  IF w = 1 AND INSTR(p, "/") = 0 THEN Result = WatchItemExpr(i) ELSE Result = PathName(p)
END FUNCTION

FUNCTION ItemRef(w AS INTEGER, i AS INTEGER) AS INTEGER
  IF i < 0 OR i > MAXVARITEMS THEN EXIT FUNCTION
  IF w = 0 THEN Result = VarItemRef(i) ELSE Result = WatchItemRef(i)
END FUNCTION

' (one of a hint's parts: "readonly", "choices:…"'s list after its colon)
FUNCTION HintHas(Hint AS STRING, Part AS STRING) AS INTEGER
  Result = Flag(INSTR("," + Hint + ",", "," + Part + ",") > 0)
END FUNCTION

FUNCTION HintAfter(Hint AS STRING, Prefix AS STRING) AS STRING
  DIM n AS INTEGER, k AS INTEGER, part AS STRING
  n = TALLY(Hint, ",") + 1
  FOR k = 1 TO n
    part = FIELD$(Hint, ",", k)
    IF LEFT$(part, LEN(Prefix)) = Prefix THEN Result = MID$(part, LEN(Prefix) + 1): EXIT FUNCTION
  NEXT
END FUNCTION

' (a kind that holds text, numbers, True / False)
FUNCTION TextKind(Kind AS STRING) AS INTEGER
  Result = Flag(LCASE$(Kind) = "string" OR LCASE$(Kind) = "str")
END FUNCTION

FUNCTION NumberKind(Kind AS STRING) AS INTEGER
  DIM k AS STRING
  k = "|" + LCASE$(Kind) + "|"
  Result = Flag(INSTR("|integer|long|byte|word|dword|short|double|single|i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|", k) > 0)
END FUNCTION

' (what was typed reads as a number: 42, -3.5, 1E3, &HFF)
FUNCTION LooksNumber(T AS STRING) AS INTEGER
  DIM i AS INTEGER, c AS STRING, digits AS INTEGER
  T = LTRIM$(RTRIM$(T))
  IF T = "" THEN EXIT FUNCTION
  IF UCASE$(LEFT$(T, 2)) = "&H" THEN
    FOR i = 3 TO LEN(T)
      IF INSTR("0123456789ABCDEFabcdef", MID$(T, i, 1)) = 0 THEN EXIT FUNCTION
    NEXT
    Result = Flag(LEN(T) > 2)
    EXIT FUNCTION
  END IF
  FOR i = 1 TO LEN(T)
    c = MID$(T, i, 1)
    IF INSTR("0123456789", c) > 0 THEN digits = digits + 1
    IF INSTR("0123456789.+-eE", c) = 0 THEN EXIT FUNCTION
  NEXT
  Result = Flag(digits > 0)
END FUNCTION

' (text as BASIC writes it: in quotes, a quote doubled, its line breaks
' and tabs as CHR$(n) between its parts)
FUNCTION TextLiteral(T AS STRING) AS STRING
  DIM i AS INTEGER, c AS STRING, run AS STRING, out AS STRING
  FOR i = 1 TO LEN(T)
    c = MID$(T, i, 1)
    IF ASC(c) < 32 THEN
      IF run <> "" THEN
        IF out <> "" THEN out = out + " + "
        out = out + CHR$(34) + run + CHR$(34)
        run = ""
      END IF
      IF out <> "" THEN out = out + " + "
      out = out + "CHR$(" + Num(ASC(c)) + ")"
    ELSEIF c = CHR$(34) THEN
      run = run + CHR$(34) + CHR$(34)
    ELSE
      run = run + c
    END IF
  NEXT
  IF run <> "" OR out = "" THEN
    IF out <> "" THEN out = out + " + "
    out = out + CHR$(34) + run + CHR$(34)
  END IF
  Result = out
END FUNCTION

' (a colour typed as the web writes it, #RRGGBB: RapidQ's &HBBGGRR number)
FUNCTION WebColour(T AS STRING) AS STRING
  DIM h AS STRING
  h = MID$(LTRIM$(RTRIM$(T)), 2)
  IF LEN(h) <> 6 OR LooksNumber("&H" + h) = 0 THEN EXIT FUNCTION
  Result = "&H" + MID$(h, 5, 2) + MID$(h, 3, 2) + MID$(h, 1, 2)
END FUNCTION

' What was typed for a value of this kind, as the expression that sets it,
' or Problem in plain words. "=" first: an expression as it is (total * 2).
' Text is text (quotes are added); numbers are numbers (or a constant's
' name: clRed); True / False take yes / no, on / off too; a property's
' constants by name.
FUNCTION TypedToExpr(Nm AS STRING, Kind AS STRING, Hint AS STRING, Typed AS STRING, BYREF Problem AS STRING) AS STRING
  DIM t AS STRING, low AS STRING, choices AS STRING, n AS INTEGER, k AS INTEGER
  Problem = ""
  t = Typed
  IF LEFT$(LTRIM$(t), 1) = "=" THEN
    Result = MID$(LTRIM$(t), 2)
    IF LTRIM$(Result) = "" THEN Problem = TR$("Type an expression after the =, such as ={name} * 2.", "name", Nm)
    EXIT FUNCTION
  END IF
  IF TextKind(Kind) THEN
    Result = TextLiteral(t)
    EXIT FUNCTION
  END IF
  t = LTRIM$(RTRIM$(t))
  low = LCASE$(t)
  IF t = "" THEN Problem = TR$("Type the new value of {name}.", "name", Nm): EXIT FUNCTION
  choices = HintAfter(Hint, "choices:")
  IF choices <> "" THEN
    n = TALLY(choices, "|") + 1
    FOR k = 1 TO n
      IF LCASE$(FIELD$(choices, "|", k)) = low THEN Result = FIELD$(choices, "|", k): EXIT FUNCTION
    NEXT
    IF LooksNumber(t) THEN Result = t: EXIT FUNCTION
    Problem = TR$("{name} takes one of: {choices}.", "name", Nm, "choices", REPLACESUBSTR$(choices, "|", ", "))
    EXIT FUNCTION
  END IF
  IF HintHas(Hint, "bool") THEN
    IF low = "on" OR low = "yes" OR low = "true" THEN Result = "1": EXIT FUNCTION
    IF low = "off" OR low = "no" OR low = "false" THEN Result = "0": EXIT FUNCTION
    IF LooksNumber(t) THEN Result = t: EXIT FUNCTION
    Problem = TR$("{name} is on or off: type on, off, 1 or 0.", "name", Nm)
    EXIT FUNCTION
  END IF
  IF LCASE$(Kind) = "boolean" OR LCASE$(Kind) = "bool" THEN
    IF low = "true" OR low = "yes" OR low = "on" OR low = "-1" OR low = "1" THEN Result = "True": EXIT FUNCTION
    IF low = "false" OR low = "no" OR low = "off" OR low = "0" THEN Result = "False": EXIT FUNCTION
    Problem = TR$("{name} is True or False: type True or False.", "name", Nm)
    EXIT FUNCTION
  END IF
  IF NumberKind(Kind) OR HintHas(Hint, "color") OR HintHas(Hint, "handle") THEN
    IF LooksNumber(t) THEN Result = t: EXIT FUNCTION
    IF HintHas(Hint, "color") AND LEFT$(t, 1) = "#" THEN
      Result = WebColour(t)
      IF Result = "" THEN Problem = TR$("A colour as the web writes it has six hex digits, such as #FF8000.")
      EXIT FUNCTION
    END IF
    ' (a constant's name — clRed, MAXORDERS —: the program says whether it is one)
    IF INSTR("ABCDEFGHIJKLMNOPQRSTUVWXYZ_", UCASE$(LEFT$(t, 1))) > 0 AND INSTR(t, " ") = 0 THEN Result = t: EXIT FUNCTION
    IF HintHas(Hint, "color") THEN
      Problem = TR$("{name} is a colour: type a number, a colour's name such as clRed, or #RRGGBB.", "name", Nm)
    ELSE
      Problem = TR$("{name} holds a number: type one, such as 42.", "name", Nm)
    END IF
    EXIT FUNCTION
  END IF
  ' (anything else — a variant not set yet —: as typed)
  Result = t
END FUNCTION

' Which editor sets a value of this kind (S11). The code's helpers bring
' their own value editors (docs/code-helpers-plan.md E1: a colour's wheel,
' a font's list, a choice's): one that takes this kind answers its name
' here, and OpenValueEditor opens it. Today: "choice" (a property's
' constants, a menu), "text" (a STRING: its whole text fetched first, in
' place or — long, of several lines — in its own window), "inline" (typed
' in place, checked before it's set).
FUNCTION ValueEditorFor(Kind AS STRING, Hint AS STRING) AS STRING
  IF HintAfter(Hint, "choices:") <> "" THEN
    Result = "choice"
  ELSEIF TextKind(Kind) THEN
    Result = "text"
  ELSE
    Result = "inline"
  END IF
END FUNCTION

' F2, a double click or Set Value on tree w's item i: its value set, as
' its kind wants — or why it can't be, in plain words.
SUB BeginSetValue(w AS INTEGER, i AS INTEGER)
  DIM expr AS STRING, kind AS STRING, hint AS STRING, nm AS STRING, editor AS STRING
  IF i < 0 OR i > MAXVARITEMS THEN EXIT SUB
  expr = ItemExpr(w, i): kind = ItemKind(w, i): hint = ItemHint(w, i): nm = ItemName(w, i)
  IF RunState <> "paused" THEN
    SetRefused nm, TR$("Values can be changed while the program is paused (F9 sets a breakpoint).")
    EXIT SUB
  END IF
  IF expr = "" THEN
    IF nm <> "" THEN SetRefused nm, TR$("{name} isn't a value that can be set.", "name", nm)
    EXIT SUB
  END IF
  IF HintHas(hint, "readonly") THEN
    SetRefused nm, TR$("{name} can't be changed: it's read-only.", "name", nm)
    EXIT SUB
  END IF
  IF ItemRef(w, i) > 0 THEN
    IF LCASE$(kind) = "array" THEN
      SetRefused nm, TR$("{name} is an array: open it and set its elements one at a time.", "name", nm)
    ELSEIF LEFT$(ItemValue(w, i), 1) = CHR$(34) THEN
      SetRefused nm, TR$("{name} is a component: open it and set its properties.", "name", nm)
    ELSE
      SetRefused nm, TR$("{name} is a record (a TYPE): open it and set its fields one at a time.", "name", nm)
    END IF
    EXIT SUB
  END IF
  EditW = w: EditI = i
  EditOld = ItemValue(w, i)
  editor = ValueEditorFor(kind, hint)
  OpenValueEditor editor
END SUB

SUB OpenValueEditor(Editor AS STRING)
  DIM v AS STRING
  SELECT CASE Editor
    CASE "choice"
      ShowChoices
    CASE "text"
      ' (never the shortened text shown: the whole of it, asked for first)
      EditFullId = Session.Evaluate(ItemExpr(EditW, EditI), "clipboard")
    CASE ELSE
      v = ItemValue(EditW, EditI)
      ' (a constant's number shown after its name: the number)
      IF LooksNumber(v) = 0 AND HintAfter(ItemHint(EditW, EditI), "const:") <> "" THEN v = HintAfter(ItemHint(EditW, EditI), "const:")
      OpenInline v
  END SELECT
END SUB

SUB OpenInline(Text AS STRING)
  ' (what the keys do, short enough for the bar)
  SetState RunState, "Enter sets it, Esc leaves it"
  ' (the edit that ends is a value's, not a watch's expression)
  EditingValue = 1
  ' (the tree keeps the focus it has: F2, a double click and its menu are
  ' all the tree's own)
  IF EditW = 0 THEN
    VarsTree.EditItem EditI, Text
  ELSE
    WatchTree.EditItem EditI, Text
  END IF
END SUB

' (a STRING's whole text in: in place when short, in its own window when
' long or of several lines; its old value remembered for Undo)
SUB EditFullText(Full AS STRING)
  ' (a credential the program holds comes as •••• and stays unseen: never
  ' edited from its mask)
  IF Full = "••••" THEN
    SetRefused ItemName(EditW, EditI), TR$("{name} holds a secret: it isn't shown, so it can't be edited here.", "name", ItemName(EditW, EditI))
    EXIT SUB
  END IF
  EditOld = TextLiteral(Full)
  EditFull = Full
  IF LEN(Full) > 80 OR INSTR(Full, CHR$(10)) > 0 OR INSTR(Full, CHR$(13)) > 0 THEN
    ValueEditor.Caption = TR$("Set {name}", "name", ItemName(EditW, EditI))
    ValueEditorBox.Text = Full
    ValueEditorInfo.Caption = TextSize(LEN(Full), TALLY(Full, CHR$(10)) + 1)
    ' (Set's click sets it — ValueEditorSetClick —; Cancel, Esc and the
    ' window's close leave it)
    ValueEditor.ShowModal
  ELSE
    OpenInline Full
  END IF
END SUB

' (the long text's window: Set — what it holds now set, the window closed)
SUB ValueEditorSetClick
  DIM t AS STRING
  ValueEditor.Close
  ' (the box's line breaks are CR LF: the text's own kept — LF alone when
  ' it had no CR)
  t = ValueEditorBox.Text
  IF INSTR(EditFull, CHR$(13)) = 0 THEN t = REPLACESUBSTR$(t, CHR$(13) + CHR$(10), CHR$(10))
  SetTyped EditW, EditI, t
END SUB

' (a value not set, and why in plain words: the reason in Output, beside
' Variables in the debugging layout — whole, where the status bar would
' shorten it —, the status bar pointing there)
SUB SetRefused(Nm AS STRING, Why AS STRING)
  SetState RunState, TR$("Can't set {name}: see Output", "name", Nm)
  OutputAfterProgram Why
END SUB

' (what was typed for tree w's item i: checked, then set)
SUB SetTyped(w AS INTEGER, i AS INTEGER, Typed AS STRING)
  DIM problem AS STRING, e AS STRING
  e = TypedToExpr(ItemName(w, i), ItemKind(w, i), ItemHint(w, i), Typed, problem)
  IF problem <> "" THEN
    SetRefused ItemName(w, i), problem
    EXIT SUB
  END IF
  ' (left as it was: nothing to set, nothing to undo)
  IF w = EditW AND i = EditI AND e = EditOld THEN
    SetState RunState, TR$("{name} is unchanged.", "name", ItemName(w, i))
    EXIT SUB
  END IF
  SetValue ItemExpr(w, i), e, ""
END SUB

' (a property's constants: a menu at the item, the one it has now checked)
SUB ShowChoices
  DIM choices AS STRING, n AS INTEGER, k AS INTEGER, now AS STRING, y AS INTEGER
  choices = HintAfter(ItemHint(EditW, EditI), "choices:")
  now = HintAfter(ItemHint(EditW, EditI), "const:")
  n = TALLY(choices, "|") + 1
  IF n > MAXCHOICES THEN n = MAXCHOICES
  FOR k = 0 TO MAXCHOICES - 1
    IF k < n THEN
      ChoiceValue(k) = FIELD$(choices, "|", k + 1)
      ChoiceItem(k).Caption = ChoiceValue(k)
      ChoiceItem(k).Checked = Flag(LCASE$(ChoiceValue(k)) = LCASE$(now))
      ChoiceItem(k).Visible = 1
    ELSE
      ChoiceItem(k).Visible = 0
    END IF
  NEXT
  ' (a flow's choice:<name>, said before: that one, without the menu — a
  ' popup menu holds the program until it's closed)
  IF ChoiceWanted <> "" THEN
    FOR k = 0 TO n - 1
      IF LCASE$(ChoiceValue(k)) = LCASE$(ChoiceWanted) THEN
        ChoiceWanted = ""
        ChoicePicked ChoiceItem(k)
        EXIT SUB
      END IF
    NEXT
    ChoiceWanted = ""
  END IF
  ' (under the item's row, right of its name and value: both still showing)
  IF EditW = 0 THEN
    y = VarsTree.Item(EditI).Top + VarsTree.ItemHeight
    IF y < VarsTree.ItemHeight THEN y = VarsTree.ItemHeight
    ChoiceMenu.PopupAt VarsTree, 220, y
  ELSE
    y = WatchTree.Item(EditI).Top + WatchTree.ItemHeight
    IF y < WatchTree.ItemHeight THEN y = WatchTree.ItemHeight
    ChoiceMenu.PopupAt WatchTree, 220, y
  END IF
END SUB

SUB ChoicePicked(Sender AS RMenuItem)
  DIM k AS INTEGER
  k = Sender.Tag
  IF k < 0 OR k >= MAXCHOICES THEN EXIT SUB
  ' (said by its name, as the menu had it)
  SetValue ItemExpr(EditW, EditI), ChoiceValue(k), ChoiceValue(k)
END SUB

' (Ctrl+Z — ⌘Z — in Variables or Watch: the trees' OnUndo)
SUB TreeUndo
  IF RunState = "paused" THEN UndoSetValue
END SUB


' Ctrl+Z in Variables or Watch while paused: the last value set goes back.
SUB UndoSetValue
  IF UndoCount < 1 THEN
    SetState RunState, "Nothing to undo"
    EXIT SUB
  END IF
  IF RunState <> "paused" THEN EXIT SUB
  UndoCount = UndoCount - 1
  SetVarExpr = UndoExpr(UndoCount)
  SetVarOld = ""
  SetVarUndo = 1
  SetVarImm = 0
  SetVarId = Session.SetVariable(UndoExpr(UndoCount), UndoOld(UndoCount))
END SUB

' (the program went on: what was set stays, its undo goes)
SUB ForgetSetValues
  UndoCount = 0
END SUB

' (Immediate: `name = value`, a place set — not `PRINT x = 5`, not `a == b`)
FUNCTION ImmAssignment(Typed AS STRING, BYREF Target AS STRING, BYREF Value AS STRING) AS INTEGER
  DIM t AS STRING, i AS INTEGER, c AS STRING, depth AS INTEGER, quoted AS INTEGER, at AS INTEGER
  t = LTRIM$(RTRIM$(Typed))
  IF t = "" OR LEFT$(t, 1) = "?" THEN EXIT FUNCTION
  FOR i = 1 TO LEN(t)
    c = MID$(t, i, 1)
    IF c = CHR$(34) THEN quoted = 1 - quoted
    IF quoted = 0 THEN
      IF c = "(" THEN depth = depth + 1
      IF c = ")" THEN depth = depth - 1
      IF c = "=" AND depth = 0 THEN at = i: EXIT FOR
      ' (a place's name: letters, digits, _ $ . and what's in brackets)
      IF depth = 0 AND INSTR("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_$%&!#.() ", c) = 0 THEN EXIT FUNCTION
    END IF
  NEXT
  IF at < 2 OR at >= LEN(t) THEN EXIT FUNCTION
  IF MID$(t, at + 1, 1) = "=" OR INSTR("<>", MID$(t, at - 1, 1)) > 0 THEN EXIT FUNCTION
  Target = RTRIM$(LEFT$(t, at - 1))
  Value = LTRIM$(MID$(t, at + 1))
  ' (one word, or a path: never "PRINT x")
  IF INSTR(Target, " ") > 0 AND INSTR(Target, "(") = 0 THEN EXIT FUNCTION
  IF UCASE$(LEFT$(Target, 4)) = "LET " THEN EXIT FUNCTION
  Result = Flag(Target <> "" AND Value <> "")
END FUNCTION

' (a value with no children, in a paused program, that has a name to set
' it by: F2 sets it as its kind wants — the editor the kernel would open
' isn't the one wanted)
SUB VarsEditing(Index AS INTEGER, AllowEdit AS INTEGER)
  AllowEdit = 0
  IF Index < 0 OR Index > MAXVARITEMS THEN EXIT SUB
  BeginSetValue 0, Index
END SUB

' (the editor held the value only: what was typed is checked and set; the
' row keeps its text until the program answers with the value it now has)
SUB VarsEdited(Index AS INTEGER, S AS STRING)
  DIM v AS STRING
  IF Index < 0 OR Index > MAXVARITEMS THEN EXIT SUB
  v = S
  S = VarsTree.Item(Index).Text
  SetTyped 0, Index, v
END SUB

' (a double click on a value: set it)
SUB VarsDblClick
  DIM i AS INTEGER
  i = VarsTree.ItemIndex
  IF i < 0 OR i > MAXVARITEMS THEN EXIT SUB
  IF VarItemRef(i) = 0 AND VarItemMore(i) < 0 AND VarItemExpr(i) <> "" THEN BeginSetValue 0, i
END SUB

SUB WatchEditing(Index AS INTEGER, AllowEdit AS INTEGER)
  DIM p AS STRING
  AllowEdit = 0
  IF Index < 0 OR Index > MAXVARITEMS THEN EXIT SUB
  p = WatchItemPath(Index)
  IF LEFT$(p, 1) <> "W" THEN EXIT SUB
  IF INSTR(p, "/") = 0 THEN
    ' (a watch's expression: always)
    AllowEdit = 1
    EditingValue = 0
  ELSEIF RunState = "paused" AND WatchItemRef(Index) = 0 AND WatchItemMore(Index) < 0 AND WatchItemExpr(Index) <> "" THEN
    IF PlainWatch(WatchItemExpr(Index)) THEN BeginSetValue 1, Index
  END IF
END SUB

SUB WatchDblClick
  DIM i AS INTEGER
  i = WatchTree.ItemIndex
  IF i < 0 OR i > MAXVARITEMS THEN EXIT SUB
  IF INSTR(WatchItemPath(i), "/") > 0 AND WatchItemRef(i) = 0 AND WatchItemMore(i) < 0 AND PlainWatch(WatchItemExpr(i)) THEN BeginSetValue 1, i
END SUB

SUB WatchEdited(Index AS INTEGER, S AS STRING)
  DIM p AS STRING, k AS INTEGER, n AS INTEGER, e AS STRING, acc AS STRING, i AS INTEGER, v AS STRING
  IF Index < 0 OR Index > MAXVARITEMS THEN EXIT SUB
  p = WatchItemPath(Index)
  k = VAL(MID$(p, 2))
  ' (Set Value… on a watch's own line: its value, not its expression)
  IF EditingValue AND EditW = 1 AND EditI = Index THEN
    EditingValue = 0
    v = S
    S = WatchTree.Item(Index).Text
    SetTyped 1, Index, v
    EXIT SUB
  END IF
  IF INSTR(p, "/") = 0 THEN
    ' (its expression: what's before its value's " = ", or all of it)
    e = S
    IF RINSTR(e, " = ") > 0 THEN e = LEFT$(e, RINSTR(e, " = ") - 1)
    e = LTRIM$(RTRIM$(e))
    n = TALLY(Session.Watches, CHR$(10))
    FOR i = 1 TO n
      IF i <> k THEN
        acc = acc + WatchExpr(i) + CHR$(10)
      ELSEIF e <> "" THEN
        acc = acc + e + CHR$(10)
      END IF
    NEXT
    Session.Watches = acc
    WatchOpen = ""
    FillWatches
    S = WatchTree.Item(Index).Text
    DebugStateChanged
  ELSE
    v = S
    S = WatchTree.Item(Index).Text
    SetTyped 1, Index, v
  END IF
END SUB

' ---- Watch ------------------------------------------------------------------------------------

SUB AddWatch(Expr AS STRING)
  DIM e AS STRING
  e = LTRIM$(RTRIM$(Expr))
  IF e = "" THEN EXIT SUB
  Session.Watches = Session.Watches + e + CHR$(10)
  FillWatches
  NeedDebugPanes
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
    NeedDebugPanes
    Dock.FocusPane("watchpanel")
    WatchEdit.SetFocus
  END IF
END SUB

' ---- Call Stack -----------------------------------------------------------------------------------

' (frame i's name as the Call Stack shows it: the program's own level in words;
' FrameName keeps "(the program)", the name the code tells it by)
FUNCTION FrameShown(i AS INTEGER) AS STRING
  IF FrameName(i) = "(the program)" THEN Result = TR$("(the program)") ELSE Result = FrameName(i)
END FUNCTION

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
    StackTree.AddItems FrameShown(k - 1) + "   " + FileTitle(FrameFile(k - 1)) + ":" + Num(FrameLine(k - 1))
    IF k = 1 THEN
      StackTree.SetItemValue k - 1, StackTree.Item(k - 1).Text, "CurrentFrame", ""
    ELSE
      StackTree.SetItemValue k - 1, StackTree.Item(k - 1).Text, "Frame", ""
    END IF
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
  DIM n AS INTEGER, typed AS STRING, t AS STRING, before AS STRING, k AS INTEGER
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
  ' (the caret's line — typed at the end, or an earlier one run again; its
  ' answer and a fresh line come below the text when it's in. The box has
  ' taken the Enter by the time this is heard: when its line break is in
  ' just before the caret, the line typed is the one above. Lines are read
  ' from the text, CR LF or LF alike — a rich edit's LineCount leaves out
  ' an empty last line)
  t = ImmediateBox.Text
  before = LEFT$(t, ImmediateBox.SelStart)
  k = TALLY(before, CHR$(10))
  IF k > 0 AND RIGHT$(before, 1) = CHR$(10) THEN k = k - 1
  typed = FIELD$(REPLACESUBSTR$(t, CHR$(13), ""), CHR$(10), k + 1)
  Key = 0
  IF LTRIM$(typed) = "" THEN EXIT SUB
  IF ImmCount < 50 THEN
    ImmHistory(ImmCount) = typed
    ImmCount = ImmCount + 1
  END IF
  ImmAt = ImmCount
  IF RunState = "paused" THEN
    IF ImmAssignment(typed, ImmSetTarget, ImmSetValue) THEN
      ' (a place set — checked, marked changed, Ctrl+Z in Variables puts it
      ' back: its old value asked for first)
      ImmOldId = Session.Evaluate(ImmSetTarget, "clipboard")
      EXIT SUB
    END IF
    ImmEvalId = Session.Evaluate(typed, "repl")
    ImmPending = ImmPending + "|" + Num(ImmEvalId) + "|"
  ELSE
    ImmAnswer TR$("(the program isn't paused: F9 sets a breakpoint, Debug > Pause stops it)")
  END IF
END SUB

' (an answer below the text — none for a statement — then a fresh line,
' the caret on it)
SUB ImmAnswer(Text AS STRING)
  DIM t AS STRING
  t = ImmediateBox.Text
  IF t <> "" AND RIGHT$(t, 1) <> CHR$(10) THEN t = t + CHR$(10)
  IF Text <> "" THEN t = t + Text + CHR$(10)
  ImmediateBox.Text = t
  ImmediateBox.SelStart = LEN(ImmediateBox.Text)
END SUB

' (an evaluation's answer: the Immediate window's, a data tip's, a value
' set, copied or viewed, Set Next Statement's; Kind its type, Ref its
' children and Count how many)
SUB SessionEvaluated(Id AS INTEGER, Answer AS STRING, Kind AS STRING, Ref AS INTEGER, Count AS INTEGER, Columns AS STRING)
  DIM failed AS INTEGER, k AS INTEGER, msg AS STRING, undoNote AS STRING
  failed = Flag(LEFT$(Answer, 6) = "error:")
  IF Id = SetVarId THEN
    SetVarId = -1
    IF failed THEN
      ' (the program's own words, plain: "count holds a whole number, …")
      IF SetVarImm THEN
        SetState RunState, TR$("Can't set {name}", "name", SetVarExpr)
        ImmAnswer LTRIM$(MID$(Answer, 7))
      ELSE
        SetRefused SetVarExpr, LTRIM$(MID$(Answer, 7))
      END IF
    ELSE
      IF SetVarUndo THEN
        SetState RunState, TR$("{name} is back to {value}", "name", SetVarExpr, "value", Answer)
      ELSE
        ' (remembered for Ctrl+Z while the program stays paused)
        IF SetVarOld <> "" THEN
          IF UndoCount >= MAXUNDO THEN
            FOR k = 1 TO MAXUNDO - 1
              UndoExpr(k - 1) = UndoExpr(k): UndoOld(k - 1) = UndoOld(k)
            NEXT
            UndoCount = MAXUNDO - 1
          END IF
          UndoExpr(UndoCount) = SetVarExpr: UndoOld(UndoCount) = SetVarOld
          UndoCount = UndoCount + 1
        END IF
        ' (what it is now; "Ctrl+Z undoes" after it when the bar has the
        ' room — the value itself is never shortened for it)
        msg = SetVarExpr + " = " + IIF(SetVarShown <> "", SetVarShown, Answer)
        IF SetVarOld <> "" THEN
          undoNote = "  " + TR$("(Ctrl+Z undoes)")
          IF FitMiddle(msg + undoNote, StateRoom) = msg + undoNote THEN msg = msg + undoNote
        END IF
        SetState RunState, msg
      END IF
      ' (Immediate: what it holds now only when that isn't what was typed —
      ' 7.9 into a whole number is 7; the line typed says the rest)
      IF SetVarImm AND Answer <> LTRIM$(RTRIM$(ImmSetValue)) THEN ImmAnswer SetVarExpr + " = " + Answer
    END IF
    SetVarUndo = 0: SetVarImm = 0: SetVarShown = ""
  ELSEIF Id = EditFullId THEN
    ' (a STRING's whole text, to edit: never the shortened one shown)
    EditFullId = -1
    IF failed THEN
      SetRefused ItemName(EditW, EditI), LTRIM$(MID$(Answer, 7))
    ELSE
      EditFullText Answer
    END IF
  ELSEIF Id = ImmOldId THEN
    ' (Immediate's `x = 5`: its old value in, now the set — remembered for Ctrl+Z)
    ImmOldId = -1
    SetVarExpr = ImmSetTarget
    SetVarOld = ""
    IF failed = 0 THEN
      IF LCASE$(Kind) = "string" THEN SetVarOld = TextLiteral(Answer) ELSE SetVarOld = Answer
      ' (a secret's mask is never put back as its value)
      IF Answer = "••••" THEN SetVarOld = ""
    END IF
    SetVarUndo = 0
    SetVarImm = 1
    SetVarId = Session.SetVariable(ImmSetTarget, ImmSetValue)
  ELSEIF INSTR(ImmPending, "|" + Num(Id) + "|") > 0 THEN
    ' (every line's answer, however fast they were typed)
    ImmPending = REPLACESUBSTR$(ImmPending, "|" + Num(Id) + "|", "")
    ImmAnswer Answer
  ELSEIF Id = HoverEvalId THEN
    IF failed = 0 AND HoverDoc >= 0 AND HoverDoc < DocCount THEN
      IF DocOpen(HoverDoc) THEN
        TipExpr = HoverExpr: TipValue = Answer: TipKind = Kind: TipRef = Ref: TipRowCount = 0
        TipChanged = TipValueChanged(TipExpr, Answer)
        ' (a value with children shows them at once, one level)
        IF Ref > 0 THEN TipOpen = "|T|" ELSE TipOpen = ""
        TipTotal = Count
        TipColumns = Columns
        ShowTip
      END IF
    END IF
  ELSEIF Id = CopyEvalId THEN
    CopyEvalId = -1
    IF failed THEN
      SetState RunState, TR$("Can't copy: {error}", "error", LTRIM$(MID$(Answer, 7)))
    ELSE
      Clipboard.Text = Answer
      SetState RunState, TRN$("Copied the value ({n} character)", "Copied the value ({n} characters)", LEN(Answer))
    END IF
  ELSEIF Id = ViewEvalId THEN
    ViewEvalId = -1
    IF failed THEN
      SetState RunState, TR$("Can't show {expr}: {error}", "expr", ViewExpr, "error", LTRIM$(MID$(Answer, 7)))
    ELSE
      ShowTextViewer ViewExpr, Answer
    END IF
  ELSEIF Id = GotoEvalId THEN
    GotoEvalId = -1
    IF failed THEN
      SetState RunState, TR$("Set Next Statement: {error}", "error", LTRIM$(MID$(Answer, 7)))
      OutputAfterProgram TR$("Set Next Statement: {error}", "error", LTRIM$(MID$(Answer, 7)))
    END IF
  END IF
END SUB

' ---- data tips: a resting mouse's value while paused, as a tree -----------------------------------

' (a name the editor found — WordAt — that isn't a value to show: none, a
' number, a word of the language)
FUNCTION NotAValue(Name AS STRING) AS INTEGER
  Result = 1
  IF Name = "" THEN EXIT FUNCTION
  IF INSTR("0123456789", LEFT$(Name, 1)) > 0 THEN EXIT FUNCTION
  IF INSTR("|IF|THEN|ELSE|ELSEIF|END|SUB|FUNCTION|DIM|AS|FOR|TO|NEXT|DO|LOOP|WHILE|WEND|SELECT|CASE|AND|OR|NOT|MOD|XOR|CREATE|PRINT|RETURN|EXIT|STEP|UNTIL|INTEGER|STRING|DOUBLE|SINGLE|LONG|BYTE|WORD|DWORD|SHORT|CALL|GOTO|GOSUB|", "|" + UCASE$(Name) + "|") > 0 THEN EXIT FUNCTION
  Result = 0
END FUNCTION

SUB CodeHoverRequest(LineNo AS INTEGER, ColNo AS INTEGER, Sender AS RCodeEditor)
  DIM d AS INTEGER, e AS STRING
  IF RunState <> "paused" THEN EXIT SUB
  d = Sender.Tag
  ' (the dotted name there — Edit1.Text — lines and columns from 1, as
  ' OnHoverRequest gives them; a number or a word of the language isn't a value)
  e = Sender.WordAt(LineNo, ColNo)
  IF NotAValue(e) THEN EXIT SUB
  HoverDoc = d
  HoverExpr = e
  HoverEvalId = Session.Evaluate(e, "hover")
END SUB

' (whether a data tip's expression shows another value than at the last
' stop that showed it)
FUNCTION TipValueChanged(Expr AS STRING, Value AS STRING) AS INTEGER
  DIM k AS INTEGER, key AS STRING, old AS STRING
  key = CHR$(1) + LCASE$(Expr) + CHR$(2)
  k = INSTR(TipSeen, key)
  IF k > 0 THEN
    old = MID$(TipSeen, k + LEN(key), INSTR(k + LEN(key), TipSeen, CHR$(3)) - k - LEN(key))
    Result = Flag(old <> Value)
  END IF
  k = INSTR(TipSeenNow, key)
  IF k > 0 THEN TipSeenNow = LEFT$(TipSeenNow, k - 1) + MID$(TipSeenNow, INSTR(k, TipSeenNow, CHR$(3)) + 1)
  TipSeenNow = TipSeenNow + key + Value + CHR$(3)
END FUNCTION

' (a stop: what the data tips showed at the last one is what they compare with)
SUB TipNewStop
  DIM n AS INTEGER, k AS INTEGER, entry AS STRING, key AS STRING, at AS INTEGER
  n = TALLY(TipSeenNow, CHR$(3))
  FOR k = 1 TO n
    entry = FIELD$(TipSeenNow, CHR$(3), k)
    key = LEFT$(entry, INSTR(entry, CHR$(2)))
    at = INSTR(TipSeen, key)
    IF at > 0 THEN TipSeen = LEFT$(TipSeen, at - 1) + MID$(TipSeen, INSTR(at, TipSeen, CHR$(3)) + 1)
    TipSeen = TipSeen + entry + CHR$(3)
  NEXT
  TipSeenNow = ""
  ' (and the tip goes: its values are the last stop's)
  HideTip
END SUB

SUB HideTip
  IF TipShown AND HoverDoc >= 0 AND HoverDoc < DocCount THEN
    IF DocOpen(HoverDoc) THEN CodeDoc(HoverDoc).HideDataTip
  END IF
  TipShown = 0
END SUB

' (a row of the tip: depth, flags, its number — its id —, name, value,
' type, a table's cells)
SUB TipRow(Depth AS INTEGER, Flags AS STRING, Path AS STRING, Expr AS STRING, Nm AS STRING, Value AS STRING, Kind AS STRING, Ref AS INTEGER, Cells AS STRING, Hint AS STRING, Count AS INTEGER)
  IF TipRowCount > MAXTIPROWS THEN EXIT SUB
  TipRowPath(TipRowCount) = Path
  TipRowExpr(TipRowCount) = Expr
  TipRowKind(TipRowCount) = Kind
  TipRowValue(TipRowCount) = Value
  TipRowRef(TipRowCount) = Ref
  TipRowMore(TipRowCount) = -1
  TipRows = TipRows + Num(Depth) + CHR$(9) + Flags + CHR$(9) + Num(TipRowCount) + CHR$(9) + Nm + CHR$(9) + Value + CHR$(9) + Kind + CHR$(9) + Cells + CHR$(9) + Hint + CHR$(9) + Num(Count) + CHR$(10)
  TipRowCount = TipRowCount + 1
END SUB

' The tip's rows: the expression, then — opened — its children, depth
' first, each open child's own; children this stop hasn't fetched are
' asked for once the rows are made (OnVariables shows the tip again).
SUB ShowTip
  DIM flags AS STRING
  IF HoverDoc < 0 OR HoverDoc >= DocCount THEN EXIT SUB
  IF DocOpen(HoverDoc) = 0 OR RunState <> "paused" THEN EXIT SUB
  TipRows = ""
  TipRowCount = 0
  TipFetch = ""
  flags = ""
  IF TipRef > 0 THEN flags = flags + "e"
  IF TipRef > 0 AND IsOpen(TipOpen, "T") THEN flags = flags + "o"
  IF TipChanged THEN flags = flags + "c"
  TipRow 0, flags, "T", TipExpr, TipExpr, TipValue, TipKind, TipRef, "", "", TipTotal
  IF TipRef > 0 AND IsOpen(TipOpen, "T") THEN TipChildren TipRef, "T", 1, TipColumns, TipTotal
  CodeDoc(HoverDoc).ShowDataTip TipRows
  TipShown = 1
  FetchTipRefs
END SUB

SUB TipChildren(Ref AS INTEGER, Path AS STRING, Depth AS INTEGER, Columns AS STRING, Total AS INTEGER)
  DIM rows AS STRING, n AS INTEGER, k AS INTEGER, row AS STRING, nm AS STRING, cref AS INTEGER, cpath AS STRING, flags AS STRING, cells AS STRING, cols AS STRING
  IF Depth > 16 THEN EXIT SUB
  IF RefIn(Ref) = 0 THEN
    TipFetch = TipFetch + "|" + Num(Ref)
    TipRow Depth, "", Path + "/…", "", "…", "", "", 0, "", "", 0
    EXIT SUB
  END IF
  rows = Session.Variables(Ref)
  n = TALLY(rows, CHR$(10))
  ' (a table: its columns' names over its rows)
  cols = Columns
  IF cols <> "" AND n > 0 THEN TipRow Depth, "h", Path + "/[columns]", "", "", "", "", 0, cols, "", 0
  FOR k = 1 TO n
    IF TipRowCount > MAXTIPROWS THEN EXIT SUB
    row = FIELD$(rows, CHR$(10), k)
    nm = FIELD$(row, CHR$(9), 1)
    cref = VAL(FIELD$(row, CHR$(9), 4))
    cells = FIELD$(row, CHR$(9), 9)
    cpath = Path + "/" + nm
    flags = ""
    IF cref > 0 THEN flags = "e"
    IF cref > 0 AND IsOpen(TipOpen, cpath) THEN flags = flags + "o"
    IF VAL(FIELD$(row, CHR$(9), 7)) THEN flags = flags + "c"
    IF cells <> "" THEN flags = flags + "t"
    TipRow Depth, flags, cpath, FIELD$(row, CHR$(9), 6), nm, FIELD$(row, CHR$(9), 2), FIELD$(row, CHR$(9), 3), cref, cells, FIELD$(row, CHR$(9), 10), VAL(FIELD$(row, CHR$(9), 5))
    IF cref > 0 AND IsOpen(TipOpen, cpath) THEN TipChildren cref, cpath, Depth + 1, FIELD$(row, CHR$(9), 8), VAL(FIELD$(row, CHR$(9), 5))
  NEXT
  ' (more than came: the next page when it's clicked)
  IF Total > n AND n > 0 THEN
    TipRow Depth, "e", Path + "/[more]", "", "…", TR$("{n} more", "n", Total - n), "", Ref, "", "", 0
    TipRowMore(TipRowCount - 1) = n
  END IF
END SUB

SUB FetchTipRefs
  DIM want AS STRING, n AS INTEGER, k AS INTEGER, r AS INTEGER
  want = TipFetch
  TipFetch = ""
  n = TALLY(want, "|")
  FOR k = 2 TO n + 1
    r = VAL(FIELD$(want, "|", k))
    IF r > 0 AND RefIn(r) = 0 THEN Session.Expand r
  NEXT
END SUB

' A tip's triangle clicked (or "… more"): opened or closed, shown again.
SUB TipExpand(Id AS STRING, Opened AS INTEGER, Sender AS RCodeEditor)
  DIM k AS INTEGER, p AS STRING
  k = VAL(Id)
  IF k < 0 OR k >= TipRowCount THEN EXIT SUB
  IF TipRowMore(k) >= 0 THEN
    Session.ExpandMore TipRowRef(k), TipRowMore(k)
    EXIT SUB
  END IF
  p = TipRowPath(k)
  IF Opened THEN
    IF INSTR(TipOpen, "|" + p + "|") = 0 THEN TipOpen = TipOpen + "|" + p + "|"
  ELSE
    TipOpen = REPLACESUBSTR$(TipOpen, "|" + p + "|", "")
  END IF
  ShowTip
END SUB

' A tip's button pressed: copy the value, watch it, or see a string whole.
SUB TipAction(Id AS STRING, Action AS STRING, Sender AS RCodeEditor)
  DIM k AS INTEGER
  k = VAL(Id)
  IF k < 0 OR k >= TipRowCount THEN EXIT SUB
  ValueAction Action, TipRowExpr(k), TipRowValue(k)
END SUB

' Copy, Watch or View for a value named by Expr (shown as Shown): its
' whole text asked for (Copy, View), or the expression added to the
' watches. A value without a name (a frame's row) copies what shows.
SUB ValueAction(Action AS STRING, Expr AS STRING, Shown AS STRING)
  SELECT CASE Action
    CASE "copy"
      IF Expr = "" THEN
        Clipboard.Text = Shown
        SetState RunState, "Copied the value"
      ELSE
        CopyEvalId = Session.Evaluate(Expr, "clipboard")
      END IF
    CASE "copyexpr"
      IF Expr <> "" THEN
        Clipboard.Text = Expr
        SetState RunState, TR$("Copied {expr}", "expr", Expr)
      END IF
    CASE "watch"
      IF Expr <> "" THEN AddWatch Expr
    CASE "view"
      IF Expr <> "" THEN
        ViewExpr = Expr
        ViewEvalId = Session.Evaluate(Expr, "clipboard")
      ELSE
        ShowTextViewer TR$("(value)"), Shown
      END IF
  END SELECT
END SUB

' ---- the text viewer: a string's whole text ---------------------------------------------------

' ("12 characters, 3 lines")
FUNCTION TextSize(Chars AS INTEGER, Lines AS INTEGER) AS STRING
  Result = TR$("{characters}, {lines}", "characters", TRN$("{n} character", "{n} characters", Chars), "lines", TRN$("{n} line", "{n} lines", Lines))
END FUNCTION

SUB ShowTextViewer(Expr AS STRING, Text AS STRING)
  DIM lines AS INTEGER
  lines = TALLY(Text, CHR$(10)) + 1
  TextViewer.Caption = TR$("Text of {expr}", "expr", Expr)
  TextViewerBox.Text = Text
  TextViewerInfo.Caption = TextSize(LEN(Text), lines)
  TextViewer.ShowModal
END SUB

SUB TextViewerCopyClick
  Clipboard.Text = TextViewerBox.Text
  TextViewerInfo.Caption = TR$("Copied")
END SUB

' ---- Set Next Statement, Step Into Specific -------------------------------------------------

' Debug > Set Next Statement (Ctrl+Shift+F10): the paused program goes on
' from the caret's line (within the SUB or FUNCTION it is paused in).
SUB SetNextStatementAtCaret
  DIM d AS INTEGER
  d = ActiveCode
  IF d < 0 OR RunState <> "paused" THEN EXIT SUB
  GotoEvalId = Session.SetNextStatement(DocFile(d), CodeDoc(d).CaretLine)
END SUB

' The arrow dragged in the gutter to another line: the same, from there.
SUB ArrowDragged(Kind AS STRING, FromLine AS INTEGER, ToLine AS INTEGER, Sender AS RCodeEditor)
  IF Kind <> "current" OR RunState <> "paused" THEN EXIT SUB
  GotoEvalId = Session.SetNextStatement(DocFile(Sender.Tag), ToLine)
END SUB

' Debug > Step Into Specific: the calls the line still makes, one picked
' (one alone: into it at once; none: a step over).
SUB StepIntoSpecific
  IF RunState <> "paused" THEN EXIT SUB
  Session.StepInTargets
END SUB

SUB SessionTargets(Targets AS STRING)
  DIM n AS INTEGER, k AS INTEGER, row AS STRING
  IF RunState <> "paused" THEN EXIT SUB
  n = TALLY(Targets, CHR$(10))
  IF n = 0 THEN
    SetState RunState, "This line calls no SUB or FUNCTION: stepping over it"
    StepStarted = TIMER
    Session.StepOver
    EXIT SUB
  END IF
  IF n = 1 THEN
    StepStarted = TIMER
    Session.StepInTarget VAL(FIELD$(Targets, CHR$(9), 1))
    EXIT SUB
  END IF
  ' (a flow's into:<name>: that one, without the menu)
  IF TargetWanted <> "" THEN
    FOR k = 1 TO n
      row = FIELD$(Targets, CHR$(10), k)
      IF UCASE$(FIELD$(row, CHR$(9), 2)) = UCASE$(TargetWanted) THEN
        TargetWanted = ""
        StepStarted = TIMER
        Session.StepInTarget VAL(FIELD$(row, CHR$(9), 1))
        EXIT SUB
      END IF
    NEXT
    TargetWanted = ""
  END IF
  TargetCount = 0
  FOR k = 0 TO MAXTARGETS - 1
    TargetItem(k).Visible = 0
  NEXT
  FOR k = 1 TO n
    IF TargetCount < MAXTARGETS THEN
      row = FIELD$(Targets, CHR$(10), k)
      TargetId(TargetCount) = VAL(FIELD$(row, CHR$(9), 1))
      TargetItem(TargetCount).Caption = TR$("Step into {name}", "name", FIELD$(row, CHR$(9), 2))
      TargetItem(TargetCount).Visible = 1
      TargetCount = TargetCount + 1
    END IF
  NEXT
  TargetMenu.Popup Screen.MouseX, Screen.MouseY
END SUB

SUB TargetPicked(Sender AS RMenuItem)
  IF Sender.Tag < 0 OR Sender.Tag >= TargetCount OR RunState <> "paused" THEN EXIT SUB
  StepStarted = TIMER
  Session.StepInTarget TargetId(Sender.Tag)
END SUB

' ---- the Variables and Watch panes' right-click menus -------------------------------------------
' Copy Value (Ctrl+C: the value whole), Copy Expression, Add Watch, View
' Text (a string's whole text); a menu each (VarsMenu, WatchMenu), their
' items' Tag the action and the pane (10 + the action: Watch's).

' (the picked item's expression and shown value: tree w — 0 Variables, 1 Watch)
FUNCTION PickedExpr(w AS INTEGER) AS STRING
  DIM i AS INTEGER
  IF w = 0 THEN
    i = VarsTree.ItemIndex
    IF i >= 0 AND i <= MAXVARITEMS THEN Result = VarItemExpr(i)
  ELSE
    i = WatchTree.ItemIndex
    IF i >= 0 AND i <= MAXVARITEMS THEN Result = WatchItemExpr(i)
  END IF
END FUNCTION

FUNCTION PickedShown(w AS INTEGER) AS STRING
  DIM t AS STRING
  IF w = 0 THEN t = VarsTree.SelectedItem ELSE t = WatchTree.SelectedItem
  IF INSTR(t, " = ") > 0 THEN Result = MID$(t, INSTR(t, " = ") + 3) ELSE Result = t
END FUNCTION

SUB ValueMenuOpening(w AS INTEGER)
  DIM e AS STRING, k AS INTEGER, paused AS INTEGER
  e = PickedExpr(w)
  paused = Flag(RunState = "paused")
  FOR k = 0 TO 4
    IF w = 0 THEN
      VarsMenuItem(k).Enabled = Flag(paused <> 0 AND (k = 0 OR e <> ""))
    ELSE
      WatchMenuItem(k).Enabled = Flag(paused <> 0 AND (k = 0 OR e <> ""))
    END IF
  NEXT
END SUB

SUB VarsMenuOpening
  ValueMenuOpening 0
END SUB

SUB WatchMenuOpening
  ValueMenuOpening 1
END SUB

SUB ValueMenuPicked(Sender AS RMenuItem)
  DIM w AS INTEGER, a AS INTEGER
  w = Sender.Tag \ 10
  a = Sender.Tag MOD 10
  SELECT CASE a
    CASE 0: ValueAction "copy", PickedExpr(w), PickedShown(w)
    CASE 1: ValueAction "copyexpr", PickedExpr(w), PickedShown(w)
    CASE 2: ValueAction "watch", PickedExpr(w), PickedShown(w)
    CASE 3: ValueAction "view", PickedExpr(w), PickedShown(w)
    CASE 4
      IF w = 0 THEN BeginSetValue 0, VarsTree.ItemIndex ELSE BeginSetValue 1, WatchTree.ItemIndex
  END SELECT
END SUB

' (the menus made: four items each)
SUB SetUpValueMenus
  DIM k AS INTEGER, captions(4) AS STRING
  captions(0) = TR$("&Copy Value") + CHR$(9) + "Ctrl+C"
  captions(1) = TR$("Copy &Expression")
  captions(2) = TR$("Add &Watch")
  captions(3) = TR$("&View Text...")
  captions(4) = TR$("&Set Value...") + CHR$(9) + "F2"
  FOR k = 0 TO 4
    VarsMenuItem(k).Caption = captions(k)
    VarsMenuItem(k).Tag = k
    VarsMenuItem(k).OnClick = ValueMenuPicked
    VarsMenu.AddItems VarsMenuItem(k)
    WatchMenuItem(k).Caption = captions(k)
    WatchMenuItem(k).Tag = 10 + k
    WatchMenuItem(k).OnClick = ValueMenuPicked
    WatchMenu.AddItems WatchMenuItem(k)
  NEXT
  VarsMenu.OnPopup = VarsMenuOpening
  WatchMenu.OnPopup = WatchMenuOpening
  VarsTree.PopupMenu = VarsMenu
  WatchTree.PopupMenu = WatchMenu
  ' (a property's constants to pick from: Set Value's menu, items made
  ' visible as needed)
  FOR k = 0 TO MAXCHOICES - 1
    ChoiceItem(k).Tag = k
    ChoiceItem(k).OnClick = ChoicePicked
    ChoiceItem(k).Visible = 0
    ChoiceMenu.AddItems ChoiceItem(k)
  NEXT
  ' (Step Into Specific's menu: an item for each call, made visible as needed)
  FOR k = 0 TO MAXTARGETS - 1
    TargetItem(k).Tag = k
    TargetItem(k).OnClick = TargetPicked
    TargetItem(k).Visible = 0
    TargetMenu.AddItems TargetItem(k)
  NEXT
END SUB

' (Edit's commands while Variables or Watch has the focus, before the
' document's — shell.inc's RunCommand: Copy the selected value — Ctrl+C,
' ⌘C on a Mac, comes as the tree's OnCopy: VarsCopy, WatchCopy —, Undo the last value
' set while paused — Ctrl+Z, the tree's OnUndo; True when Id was one)
FUNCTION DebugPaneCommand(Id AS STRING) AS INTEGER
  DIM w AS INTEGER
  ' (Call Stack's and Breakpoints' Copy: the selected row)
  IF Id = "edit.copy" THEN
    SELECT CASE LCASE$(Dock.ActivePane)
      CASE "stacktree": StackCopy: Result = 1: EXIT FUNCTION
      CASE "bppanel": BpCopy: Result = 1: EXIT FUNCTION
    END SELECT
  END IF
  SELECT CASE LCASE$(Dock.ActivePane)
    CASE "varstree": w = 0
    CASE "watchpanel": w = 1
    CASE ELSE: EXIT FUNCTION
  END SELECT
  SELECT CASE Id
    CASE "edit.copy"
      ValueAction "copy", PickedExpr(w), PickedShown(w)
      Result = 1
    CASE "edit.undo"
      IF RunState = "paused" THEN
        UndoSetValue
        Result = 1
      END IF
  END SELECT
END FUNCTION

' (POLISH-2) Ctrl+C — ⌘C on a Mac — in Variables or Watch (the tree's
' OnCopy, which comes from the tree that has the keyboard, whichever pane
' the dock says is active): the selected value copied, as Edit > Copy
SUB VarsCopy
  ValueAction "copy", PickedExpr(0), PickedShown(0)
END SUB

SUB WatchCopy
  ValueAction "copy", PickedExpr(1), PickedShown(1)
END SUB

' ---- Call Stack and Breakpoints: Copy, Copy All (COPY-ANYWHERE) ----------------------
DIM StackMenu AS RPopupMenu
DIM StackMenuItem(1) AS RMenuItem
DIM BpMenu AS RPopupMenu
DIM BpMenuItem(1) AS RMenuItem

' (Ctrl+C — ⌘C — in Call Stack, the tree's OnCopy, and its menu's Copy:
' the selected call as shown, "Main   main.rr:12")
SUB StackCopy
  DIM i AS INTEGER
  i = StackTree.ItemIndex
  IF i < 0 OR i >= StackTree.ItemCount THEN EXIT SUB
  Clipboard.Text = StackTree.Item(i).Text
  SetState RunState, "Copied the call"
END SUB

' (…and in Breakpoints: the selected breakpoint as listed, "main.rr:12  when x > 1")
SUB BpCopy
  DIM i AS INTEGER
  i = BpTree.ItemIndex
  IF i < 0 OR i >= BpCount THEN EXIT SUB
  Clipboard.Text = BreakpointText(i)
  SetState RunState, "Copied the breakpoint"
END SUB

' (Copy All: every call, every breakpoint, a line each)
SUB ListCopyAll(Which AS INTEGER)
  DIM i AS INTEGER, t AS STRING, n AS INTEGER
  t = ""
  IF Which = 0 THEN
    n = StackTree.ItemCount
    FOR i = 0 TO n - 1
      IF i > 0 THEN t = t + CHR$(10)
      t = t + StackTree.Item(i).Text
    NEXT
  ELSE
    n = BpCount
    FOR i = 0 TO n - 1
      IF i > 0 THEN t = t + CHR$(10)
      t = t + BreakpointText(i)
    NEXT
  END IF
  IF n = 0 THEN EXIT SUB
  Clipboard.Text = t
  IF Which = 0 THEN
    SetState RunState, TRN$("Copied {n} call", "Copied {n} calls", n)
  ELSE
    SetState RunState, TRN$("Copied {n} breakpoint", "Copied {n} breakpoints", n)
  END IF
END SUB

SUB ListMenusOpening
  StackMenuItem(0).Enabled = Flag(StackTree.ItemIndex >= 0)
  StackMenuItem(1).Enabled = Flag(StackTree.ItemCount > 0)
  BpMenuItem(0).Enabled = Flag(BpTree.ItemIndex >= 0 AND BpCount > 0)
  BpMenuItem(1).Enabled = Flag(BpCount > 0)
END SUB

SUB ListMenuPicked(Sender AS RMenuItem)
  SELECT CASE Sender.Tag
    CASE 0: StackCopy
    CASE 1: ListCopyAll 0
    CASE 10: BpCopy
    CASE 11: ListCopyAll 1
  END SELECT
END SUB

SUB SetUpListMenus
  DIM k AS INTEGER
  FOR k = 0 TO 1
    StackMenuItem(k).Caption = IIF(k = 0, TR$("&Copy"), TR$("Copy &All"))
    StackMenuItem(k).Tag = k
    StackMenuItem(k).OnClick = ListMenuPicked
    StackMenu.AddItems StackMenuItem(k)
    BpMenuItem(k).Caption = IIF(k = 0, TR$("&Copy"), TR$("Copy &All"))
    BpMenuItem(k).Tag = 10 + k
    BpMenuItem(k).OnClick = ListMenuPicked
    BpMenu.AddItems BpMenuItem(k)
  NEXT
  StackMenuItem(0).ShortCut = "Ctrl+C"
  BpMenuItem(0).ShortCut = "Ctrl+C"
  StackMenu.OnPopup = ListMenusOpening
  BpMenu.OnPopup = ListMenusOpening
  StackTree.PopupMenu = StackMenu
  BpTree.PopupMenu = BpMenu
END SUB

' ---- the commands ------------------------------------------------------------------------------
' True when Id was one of Run's or Debug's (and it ran).
FUNCTION DebugCommand(Id AS STRING) AS INTEGER
  DIM d AS INTEGER
  d = ActiveCode
  Result = 1
  IF LEFT$(Id, 10) = "debug.step" AND RunState = "paused" THEN
    StepStarted = TIMER
    ' (a step that runs on — a ShowModal stepped over — is the program's
    ' turn: its windows come forward after half a second)
    StepRaise.Enabled = 0
    StepRaise.Enabled = 1
  END IF
  ' ((S11) Edit > Undo in Variables or Watch while paused: DebugPaneCommand,
  ' before the documents' commands)
  SELECT CASE Id
    CASE "run.start"
      IF RunState = "paused" THEN ContinueProgram ELSE StartProgram 1
    CASE "run.withoutDebugging": StartProgram 0
    CASE "run.stop": StopProgram
    CASE "run.restart"
      StopProgram
      StartProgram 1
    CASE "run.browser": RunInBrowser
    CASE "run.continue": IF RunState = "paused" THEN ContinueProgram
    CASE "run.pause": IF RunState = "running" THEN Session.Pause
    CASE "debug.showProgram": ShowProgram 1
    CASE "debug.stepInto"
      ' (not running: it starts, stopping at the first line)
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
    CASE "debug.setNextStatement": SetNextStatementAtCaret
    CASE "debug.stepIntoSpecific": StepIntoSpecific
    CASE "debug.toggleBreakpoint"
      IF d >= 0 THEN ToggleBreakpointAt d, CodeDoc(d).CaretLine
    CASE "debug.editBreakpoint": EditBreakpointAtCaret
    CASE "debug.clearBreakpoints": ClearAllBreakpoints
    CASE "debug.addWatch": AddWatchCommand
    CASE "debug.breakOnErrors"
      BreakOnErrors = 1 - BreakOnErrors
      CheckCmd "debug.breakOnErrors", BreakOnErrors
      IF BreakOnErrors THEN SetState RunState, "The debugger stops at run-time errors" ELSE SetState RunState, "Run-time errors end the program"
    ' (the debug panes asked for while editing: the debugging layout, as by hand)
    CASE "view.variables"
      NeedDebugPanes
      Dock.FocusPane("varstree")
    CASE "view.watch"
      NeedDebugPanes
      Dock.FocusPane("watchpanel")
    CASE "view.callStack"
      NeedDebugPanes
      Dock.FocusPane("stacktree")
    CASE "view.breakpoints": Dock.FocusPane("bppanel")
    CASE "view.debugPanes": ToggleDebugPanes
    CASE ELSE: Result = 0
  END SELECT
END FUNCTION

' (Continue, F5 while paused: the program goes on, its windows in front —
' it's the program's turn)
SUB ContinueProgram
  Session.Continue
  ShowProgram 0
END SUB

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
'   varmenu:set       Set Value… on the picked Variables item (S11)
'   watchpick:<path>  Watch's item at that path picked (W1: the first watch); watchmenu:set its Set Value…
'   choice:<name>     (before F2) one of Set Value's constants picked from its menu
'   valueset:<text>   the long text's window: its text (\n a line break), then Set
'   valuecancel:      the long text's window: Cancel
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
        EditorFacts d
      END IF
    CASE "watch"
      WatchEdit.Text = arg
      WatchEditKey 13, 0
    CASE "varpick"
      ' (varpick:<path>: Variables' item at that path selected, the tree
      ' focused — then key:F2 edits it)
      FOR k = 0 TO VarsTree.ItemCount - 1
        IF k <= MAXVARITEMS THEN
          IF LCASE$(VarItemPath(k)) = LCASE$(arg) THEN
            VarsTree.ItemIndex = k
            ' (in view: a few rows above it show too)
            IF k > 3 THEN VarsTree.TopIndex = k - 3
          END IF
        END IF
      NEXT
      Dock.FocusPane("varstree")
    CASE "bpcond", "bphit", "bplog", "bpoff"
      IF BpPicked >= 0 THEN
        IF verb = "bpcond" THEN BpCondEdit.Text = arg
        IF verb = "bphit" THEN BpHitEdit.Text = arg
        IF verb = "bplog" THEN BpLogEdit.Text = arg
        IF verb = "bpoff" THEN BpOnCheck.Checked = 0
        ApplyBreakpointEdits
      END IF
    CASE "hover"
      ' (as a resting mouse there: the editor's hover request, at that place)
      IF d >= 0 THEN CodeDoc(d).TriggerHover VAL(FIELD$(arg, ":", 1)), VAL(FIELD$(arg, ":", 2))
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
    CASE "tipopen", "tipclose"
      ' (tipopen:<row> — a data tip's row opened by its triangle; tipclose: closed)
      IF HoverDoc >= 0 AND HoverDoc < DocCount THEN TipExpand arg, Flag(verb = "tipopen"), CodeDoc(HoverDoc)
    CASE "tipaction"
      ' (tipaction:<row>:<copy|watch|view> — a data tip row's button)
      IF HoverDoc >= 0 AND HoverDoc < DocCount THEN TipAction FIELD$(arg, ":", 1), FIELD$(arg, ":", 2), CodeDoc(HoverDoc)
    CASE "goto"
      ' (goto:<line> — Set Next Statement to that line of the active code,
      ' as the arrow dragged there)
      IF d >= 0 THEN ArrowDragged "current", PausedLine, VAL(arg), CodeDoc(d)
    CASE "into"
      ' (into:<name> — Step Into Specific, that SUB or FUNCTION picked)
      TargetWanted = arg
      StepIntoSpecific
    CASE "varmenu"
      ' (varmenu:<copy|copyexpr|watch|view|set> — the Variables' menu on the picked item)
      IF arg = "set" THEN BeginSetValue 0, VarsTree.ItemIndex ELSE ValueAction arg, PickedExpr(0), PickedShown(0)
    CASE "watchpick"
      ' (watchpick:<path> — Watch's item at that path selected, the tree
      ' focused: W1 is the first watch)
      Dock.ShowPane("watchpanel")
      FOR k = 0 TO WatchTree.ItemCount - 1
        IF k <= MAXVARITEMS THEN
          IF LCASE$(WatchItemPath(k)) = LCASE$(arg) THEN WatchTree.ItemIndex = k
        END IF
      NEXT
      WatchTree.SetFocus
    CASE "watchmenu"
      ' (watchmenu:set — Set Value… on the picked Watch item)
      IF arg = "set" THEN BeginSetValue 1, WatchTree.ItemIndex
    CASE "choice"
      ' (choice:<constant>, before F2 — Set Value's menu of a property's
      ' constants: that one picked, as its click)
      ChoiceWanted = arg
    CASE "valueset", "valuecancel"
      ' (valueset:<text> — the long text's window: its text, then Set;
      ' valuecancel: Cancel)
      IF verb = "valueset" THEN
        ValueEditorBox.Text = REPLACESUBSTR$(arg, "\n", CHR$(10))
        ValueEditorSetClick
      ELSE
        ValueEditor.Close
      END IF
    CASE "collapse"
      ' (collapse:<path> — a value in Variables closed, as a click on its - does)
      VarsOpen = REPLACESUBSTR$(VarsOpen, "|" + arg + "|", "")
      FillVariables
    CASE "changed"
      ' (changed:report — the Variables' and Watch's values marked changed,
      ' by path, in Output)
      arg = ""
      FOR k = 0 TO VarsTree.ItemCount - 1
        IF k <= MAXVARITEMS THEN
          IF VarsTree.Item(k).Changed THEN arg = arg + " " + VarItemPath(k)
        END IF
      NEXT
      FOR k = 0 TO WatchTree.ItemCount - 1
        IF k <= MAXVARITEMS THEN
          IF WatchTree.Item(k).Changed THEN arg = arg + " " + WatchItemPath(k)
        END IF
      NEXT
      OutputLine "[changed]" + arg
    CASE "perf"
      ' (perf:report — the run's and the steps' times, docs/studio-wow.md §3.15)
      OutputLine "[perf] f5-form=" + Num(PerfForm) + " step=" + Num(PerfStep)
    CASE "cmds"
      ' (cmds:report — Run's and Debug's commands as the person sees them
      ' now: the state, then each command's menu item, tool button and
      ' palette entry, 1 enabled / 0 not / - none; the Run item's caption
      ' and the ▶ button's hint. cmds:log — every report so far again, a
      ' run's start having cleared Output)
      IF arg = "log" THEN OutputLine "[cmds-log]" + CHR$(10) + CmdsLog ELSE CmdsReport
    CASE "stops"
      ' (stops:report — every stop of this run: "[stops] 58:breakpoint
      ' 58:breakpoint 59:step …", line and reason)
      OutputLine "[stops]" + StopLog
    CASE "panes"
      ' (panes:report — the first rows Call Stack and Variables show now:
      ' "[panes] Running… | Running… the values …" while it runs on —
      ' POLISH-2)
      arg = "[panes] "
      IF StackTree.ItemCount > 0 THEN arg = arg + StackTree.Item(0).Text
      arg = arg + " | "
      IF VarsTree.ItemCount > 0 THEN arg = arg + VarsTree.Item(0).Text
      OutputLine arg
    CASE "extent"
      ' (extent:<pane>:<width|height>:<pixels> — a splitter dragged so the
      ' pane's part is that size, as the user drags it — S9's sizes)
      Dock.PaneExtent(FIELD$(arg, ":", 1), FIELD$(arg, ":", 2), VAL(FIELD$(arg, ":", 3)))
    CASE ELSE: Result = 0
  END SELECT
END FUNCTION

' (cmds:report's line: "[cmds] paused@6:breakpoint run.start=111 …"; the
' lines so far, for cmds:log; this run's stops, for stops:report)
DIM CmdsLog AS STRING
DIM StopLog AS STRING

SUB CmdsReport
  DIM ids AS STRING, n AS INTEGER, k AS INTEGER, id AS STRING, i AS INTEGER, t AS STRING, line AS STRING
  ids = "run.start,run.withoutDebugging,run.stop,run.restart,run.continue,run.pause,debug.showProgram,debug.stepInto,debug.stepOver,debug.stepOut,debug.runToCursor,debug.setNextStatement,debug.stepIntoSpecific,debug.toggleBreakpoint,run.build"
  n = TALLY(ids, ",") + 1
  line = "[cmds] " + RunState
  IF RunState = "paused" THEN line = line + "@" + Num(PausedLine) + ":" + Session.StopReason
  FOR k = 1 TO n
    id = FIELD$(ids, ",", k)
    i = CmdIndex(id)
    IF i >= 0 THEN
      IF Cmd(i).Item >= 0 THEN t = Num(Flag(MI(Cmd(i).Item).Enabled)) ELSE t = "-"
      IF Cmd(i).Tool >= 0 THEN t = t + Num(Flag(ToolBar.ButtonEnabled(id))) ELSE t = t + "-"
      t = t + Num(Flag(Palette.CommandEnabled(id)))
      line = line + " " + id + "=" + t
    END IF
  NEXT
  i = CmdIndex("run.start")
  IF i >= 0 THEN
    IF Cmd(i).Item >= 0 THEN line = line + " | " + REPLACESUBSTR$(MI(Cmd(i).Item).Caption, CHR$(9), " ")
    line = line + " | " + ToolBar.ButtonHint("run.start")
  END IF
  OutputLine line
  CmdsLog = CmdsLog + line + CHR$(10)
END SUB

' ---- set-up -------------------------------------------------------------------------------------

' The debugger's trees in the chrome's font and rows, as the Outline
' (chrome.inc's ApplyFonts: at the start and when the theme changes).
SUB ApplyDebugFonts(Rows AS INTEGER)
  VarsTree.Font = UIFont: VarsTree.ItemHeight = Rows
  WatchTree.Font = UIFont: WatchTree.ItemHeight = Rows
  StackTree.Font = UIFont: StackTree.ItemHeight = Rows
  BpTree.Font = UIFont: BpTree.ItemHeight = Rows
END SUB

' The changed values' colour in the theme (chrome.inc's ApplyChrome). The
' trees draw changed values in the theme's own colours (Item(i).Changed:
' highlighted and marked), which follow a change of theme by themselves.
SUB ApplyDebugColors
  ChangedColor = Application.ThemeColor("debug.changed")
END SUB

SUB DebugTree(t AS RTreeView, Title AS STRING)
  t.Parent = Studio
  t.Width = 440: t.Height = 190
  t.ReadOnly = 1
  t.HideSelection = 0
  t.AccessibleName = Title
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
  SetVarId = -1
  EditFullId = -1: ImmOldId = -1
  Session.OnOutput = ProgramOutput
  Session.OnStopped = ProgramStopped
  Session.OnContinue = ProgramContinued
  Session.OnExit = ProgramExited
  Session.OnVariables = SessionVariables
  Session.OnEvaluate = SessionEvaluated
  Session.OnFormShown = ProgramFormShown
  Session.OnBreakpointPlaced = BreakpointPlaced
  Session.OnStepInTargets = SessionTargets
  CopyEvalId = -1
  ViewEvalId = -1
  GotoEvalId = -1
  HoverDoc = -1
  TextViewerCopy.OnClick = TextViewerCopyClick
  ValueEditorSet.OnClick = ValueEditorSetClick
  PerfForm = -1
  PerfStep = -1
  ' (RAPIDR_STUDIO_PERF: the moment Studio is up and waiting, printed for
  ' tests/studio_perf.mjs — PERF-1's cold start)
  IF ENVIRON$("RAPIDR_STUDIO_PERF") <> "" THEN PerfReady.Enabled = 1

  DebugTree VarsTree, TR$("Variables")
  VarsTree.OnExpanding = VarsExpanding
  VarsTree.OnCollapsed = VarsCollapsed
  ' (a value set in place: F2)
  VarsTree.ReadOnly = 0
  VarsTree.OnEditing = VarsEditing
  VarsTree.OnEdited = VarsEdited
  VarsTree.OnDblClick = VarsDblClick
  VarsTree.OnUndo = TreeUndo
  VarsTree.OnCopy = VarsCopy

  WatchPanel.Parent = Studio
  WatchPanel.Width = 440: WatchPanel.Height = 190
  WatchPanel.BevelOuter = 0
  WatchPanel.AccessibleName = TR$("Watch")
  ' (a strip at the top: "Add" and the box an expression is typed in)
  WatchStrip.Parent = WatchPanel
  WatchStrip.Align = 1
  WatchStrip.Height = 30
  WatchStrip.BevelOuter = 0
  WatchLabel.Parent = WatchStrip
  WatchLabel.Left = 6: WatchLabel.Top = 7: WatchLabel.Width = 88
  WatchLabel.Caption = TR$("&Add a watch:")
  WatchEdit.Parent = WatchStrip
  WatchEdit.Left = 96: WatchEdit.Top = 4: WatchEdit.Width = 260
  WatchEdit.Hint = TR$("Add a watch: an expression, then Enter")
  WatchEdit.AccessibleName = TR$("Add a watch")
  WatchEdit.OnKeyDown = WatchEditKey
  DebugTree WatchTree, TR$("Watches")
  WatchTree.Parent = WatchPanel
  WatchTree.Align = 5
  WatchTree.OnExpanding = WatchExpanding
  WatchTree.OnCollapsed = WatchCollapsed
  WatchTree.OnKeyDown = WatchTreeKey
  WatchTree.OnDblClick = WatchDblClick
  WatchTree.OnUndo = TreeUndo
  WatchTree.OnCopy = WatchCopy
  WatchTree.ReadOnly = 0
  WatchTree.OnEditing = WatchEditing
  WatchTree.OnEdited = WatchEdited

  DebugTree StackTree, TR$("Call Stack")
  StackTree.ShowRoot = 0
  StackTree.ShowLines = 0
  StackTree.ShowButtons = 0
  StackTree.OnChange = StackPicked

  BpPanel.Parent = Studio
  BpPanel.Width = 440: BpPanel.Height = 190
  BpPanel.BevelOuter = 0
  BpPanel.AccessibleName = TR$("Breakpoints")
  BpStrip.Parent = BpPanel
  BpStrip.Align = 2
  BpStrip.Height = 30
  BpStrip.BevelOuter = 0
  BpCondLabel.Parent = BpStrip
  ' (each label clear of its box: Inter is wider than MS Sans Serif)
  BpCondLabel.Left = 6: BpCondLabel.Top = 7: BpCondLabel.Width = 40
  BpCondLabel.Caption = TR$("&When")
  BpCondEdit.Parent = BpStrip
  BpCondEdit.Left = 50: BpCondEdit.Top = 4: BpCondEdit.Width = 130
  BpCondEdit.Hint = TR$("A condition: it stops when this is true (clicks > 1)")
  BpCondEdit.AccessibleName = TR$("Condition")
  BpCondEdit.OnKeyDown = BpEditKey
  BpHitLabel.Parent = BpStrip
  BpHitLabel.Left = 190: BpHitLabel.Top = 7: BpHitLabel.Width = 24
  BpHitLabel.Caption = TR$("&Hit")
  BpHitEdit.Parent = BpStrip
  BpHitEdit.Left = 218: BpHitEdit.Top = 4: BpHitEdit.Width = 50
  BpHitEdit.Hint = TR$("A hit count: 3 (the third time), >= 3, % 3 (every third)")
  BpHitEdit.AccessibleName = TR$("Hit count")
  BpHitEdit.OnKeyDown = BpEditKey
  BpLogLabel.Parent = BpStrip
  BpLogLabel.Left = 278: BpLogLabel.Top = 7: BpLogLabel.Width = 28
  BpLogLabel.Caption = TR$("&Log")
  BpLogEdit.Parent = BpStrip
  BpLogEdit.Left = 310: BpLogEdit.Top = 4: BpLogEdit.Width = 116
  BpLogEdit.Hint = TR$("A logpoint: prints this ({{expr}} shows a value) and goes on")
  BpLogEdit.AccessibleName = TR$("Log message")
  BpLogEdit.OnKeyDown = BpEditKey
  BpOnCheck.Parent = BpStrip
  BpOnCheck.Left = 436: BpOnCheck.Top = 6: BpOnCheck.Width = 46
  BpOnCheck.Caption = TR$("On")
  BpOnCheck.OnClick = BpOnClicked
  DebugTree BpTree, TR$("Breakpoints")
  BpTree.Parent = BpPanel
  BpTree.Align = 5
  BpTree.ShowRoot = 0
  BpTree.ShowLines = 0
  BpTree.ShowButtons = 0
  BpTree.OnClick = BpTreeClick
  BpTree.OnDblClick = BpTreeOpen
  BpTree.OnKeyDown = BpTreeKey
  ' (COPY-ANYWHERE: Ctrl+C — ⌘C — and a right-click's Copy / Copy All in
  ' Call Stack and Breakpoints)
  StackTree.OnCopy = StackCopy
  BpTree.OnCopy = BpCopy
  SetUpListMenus

  ' (the debugging layout's group beside Output — Variables, Watch, Call
  ' Stack, Breakpoints —; Breakpoints is a tab beside Output while editing:
  ' DebugLayoutLoaded, after the layout is read, puts each where the mode
  ' says)
  Dock.AddPane(VarsTree, TR$("Variables"), "right:outputbox", "variables")
  Dock.AddPane(WatchPanel, TR$("Watch"), "tab:varstree", "watch")
  Dock.AddPane(StackTree, TR$("Call Stack"), "tab:varstree", "callstack")
  Dock.AddPane(BpPanel, TR$("Breakpoints"), "tab:varstree", "breakpoint")
  Dock.ShowPane("varstree")
  IF DebugPanesMode = "" THEN DebugPanesMode = "auto"
  SetUpValueMenus
  FillVariables
  FillWatches
  FillBreakpoints
END SUB

' The language changed: the debugger's panes made again in it (the titles
' and what the trees show; captions, hints and names follow by themselves,
' but the logpoint's hint has braces in it and is set again here).
SUB DebugLanguageChanged
  Dock.PaneTitle("varstree", TR$("Variables"))
  Dock.PaneTitle("watchpanel", TR$("Watch"))
  Dock.PaneTitle("stacktree", TR$("Call Stack"))
  BpLogEdit.Hint = TR$("A logpoint: prints this ({{expr}} shows a value) and goes on")
  IF RunState = "paused" THEN
    FillCallStack
  ELSEIF DebugStale THEN
    TreeFilling = 1
    StackTree.Clear
    StackTree.AddItems TR$("Running…")
    TreeFilling = 0
  END IF
  FillVariables
  FillWatches
  FillBreakpoints
END SUB

' ---- kept with the project: its breakpoints and watches -----------------------------------
' In Studio's settings (the per-user store; the page's on the web) by the
' project's file, saved a moment after they change (as the layout is:
' workspace.inc) and when Studio closes: opening the project again brings
' them back. Files relative to the
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


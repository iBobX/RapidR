' versions.inc: Versions (docs/versions-plan.md, lane G3) — the project's
' versions in plain words, each word with its Git name in a tooltip, and
' nothing ever lost:
'   - the Versions pane (a tab beside Project; View > Versions): Start
'     Versioning This Project for a project without versions; then the
'     files not saved as a version, Save a Version..., the Timeline (an
'     AI's checkpoints lighter), What Changed?, Go Back..., Undo;
'   - Save a Version... (File; Ctrl+Shift+V, Cmd+Shift+V on a Mac — Ctrl+S
'     stays Save): the words, the files (every changed one ticked), the
'     check for keys and passwords said in words, saved anyway only once
'     the person ticks it;
'   - the code editors' gutter marks against the last version
'     (RCodeEditor.ChangeBase): a click on one shows what the lines were,
'     with Undo This Change;
'   - the Project pane's badges (M changed, N new, D removed: a letter and
'     words, never colour alone) and the status bar's experiment and count;
'   - the local history (RRepository.Snapshot): every save keeps a copy of
'     the file, versioned or not, in the project's .rapidr/history — Local
'     History of This File compares a copy with the editor and restores it;
'   - the web: no versions (gitoxide doesn't run in a browser), said in the
'     pane; the local history works there too.
' Every caption and message goes through TR$ (Studio is translatable).

DECLARE SUB VerLayOut
DECLARE SUB VerDiffLayOut
DECLARE SUB VerSaveLayOut
DECLARE SUB VerStartClick
DECLARE SUB VerSaveClick
DECLARE SUB VerWhatClick
DECLARE SUB VerBackClick
DECLARE SUB VerUndoClick
DECLARE SUB VerLocalClick
DECLARE SUB VerTimePicked
DECLARE SUB VerTimeOpen
DECLARE SUB VerChangePicked
DECLARE SUB VerChangeOpen
DECLARE SUB VerDrawTime(Index AS INTEGER, State AS INTEGER, R AS RRect)
DECLARE SUB VerDrawChange(Index AS INTEGER, State AS INTEGER, R AS RRect)
DECLARE SUB VerDiffPicked
DECLARE SUB VerDiffActClick
DECLARE SUB VerBackPicked(Sender AS RMenuItem)
DECLARE SUB VerSaveOkClick
DECLARE SUB VerSaveNoClick
DECLARE SUB VerMsgChanged
DECLARE SUB VerAnywayClick
DECLARE SUB VerRefresh
DECLARE SUB VerShowPane
DECLARE SUB VerShowDiff(Mode AS STRING)
DECLARE SUB VerReloadDocs
DECLARE SUB ShowSaveVersion
DECLARE FUNCTION VerAgo(Seconds AS LONG) AS STRING
DECLARE FUNCTION VerRel(Path AS STRING) AS STRING
DECLARE FUNCTION VerChangesWords AS STRING
DECLARE FUNCTION VerTickedFiles AS STRING
DECLARE FUNCTION VerSaveOpenFiles AS INTEGER
DECLARE FUNCTION VerPicked AS INTEGER
DECLARE SUB VerShowState
DECLARE SUB VerStart
DECLARE SUB VerGoBack(OneFile AS INTEGER)
DECLARE SUB VerUndo
DECLARE SUB VerWarnShow(Text AS STRING, Lines AS STRING)

CONST VERMAX = 200

' (Repo, the RRepository: decl.inc)
' (the timeline as last read: each row's id, kind, message, who, when)
DIM VerTimeId(VERMAX) AS STRING
DIM VerTimeKind(VERMAX) AS STRING
DIM VerTimeMsg(VERMAX) AS STRING
DIM VerTimeWho(VERMAX) AS STRING
DIM VerTimeWhen(VERMAX) AS LONG
DIM VerTimeCount AS INTEGER
' (the changes not saved as a version: each row's letter and path)
DIM VerChLetter(VERMAX) AS STRING
DIM VerChPath(VERMAX) AS STRING
DIM VerChCount AS INTEGER
' (the What Changed / Local History document: what it shows — "version"
' (VerDiffVersion's files), "changes" (since the last version) or
' "history" (VerDiffFile's copies) — and its rows)
DIM VerDiffMode AS STRING
DIM VerDiffVersion AS STRING
DIM VerDiffFile AS STRING
DIM VerDiffRow(VERMAX) AS STRING
DIM VerDiffRowCount AS INTEGER
' (what the badges said last: the paths that have one)
DIM VerBadged AS STRING
' (who saves the versions: Save a Version's "Your name", kept)
DIM VerAuthor AS STRING
' (Go Back...'s menu: the whole project, or the file open)
DIM VerBackMenu AS RPopupMenu
DIM VerBackItem(1) AS RMenuItem
DIM VerSaveFound AS INTEGER
DIM VerBmp(3) AS RBitmap
' (the lists' text measured: a long message cut with "…")
DIM VerMeasure AS RBitmap

' ---- the Versions pane ------------------------------------------------------------------
CREATE VerPanel AS RPanel
  BevelOuter = 0
  AccessibleName = TR$("Versions")
  ' (the header, docs/design-guidelines.md "Pane anatomy": the title and
  ' a line under it; Save a Version... compact at the right)
  CREATE VerTop AS RPanel
    Align = 1
    Height = 52
    BevelOuter = 0
    CREATE VerHead AS RLabel
      Left = 12: Top = 9: Width = 200
      Caption = TR$("Versions")
    END CREATE
    CREATE VerSub AS RLabel
      Left = 12: Top = 27: Width = 200
    END CREATE
  END CREATE
  ' (a project without versions: what they are, one button)
  CREATE VerEmptyText AS RLabel
    WordWrap = 1: AutoSize = 0
  END CREATE
  CREATE VerStartBtn AS RButton
    Caption = TR$("&Start Versioning This Project")
    Hint = TR$("Start keeping versions of this project (Git calls this git init)")
    ShowHint = 1
    OnClick = VerStartClick
  END CREATE
  CREATE VerLocalNote AS RLabel
    WordWrap = 1: AutoSize = 0
  END CREATE
  CREATE VerLocalBtn AS RButton
    Caption = TR$("&Local History...")
    Hint = TR$("The copies Studio kept of this file each time it was saved")
    ShowHint = 1
    OnClick = VerLocalClick
  END CREATE
  ' (a project with versions: its changes, its timeline, what to do)
  CREATE VerChangesHead AS RLabel
    Caption = TR$("Changes not saved as a version")
    WordWrap = 1: AutoSize = 0
  END CREATE
  CREATE VerChanges AS RListBox
    Style = lbOwnerDrawFixed
    AccessibleName = TR$("Files not saved as a version")
    AccessibleDescription = TR$("A double click opens the file")
    OnDrawItem = VerDrawChange
    OnClick = VerChangePicked
    OnChange = VerChangePicked
    OnDblClick = VerChangeOpen
  END CREATE
  CREATE VerSaveBtn AS RButton
    Caption = TR$("&Save a Version...")
    Hint = TR$("Save a version of your files (Git calls this a commit)")
    ShowHint = 1
    AccessibleDescription = TR$("Keeps the changed files as they are now, with a few words about what changed (Git calls this a commit)")
    OnClick = VerSaveClick
  END CREATE
  CREATE VerTimeHead AS RLabel
    Caption = TR$("Timeline")
    Hint = TR$("Every version, newest first (Git calls this the log)")
    ShowHint = 1
  END CREATE
  CREATE VerTimeline AS RListBox
    Style = lbOwnerDrawFixed
    AccessibleName = TR$("Timeline: every version, newest first")
    AccessibleDescription = TR$("Up and Down pick a version; What Changed? and Go Back... act on it")
    OnDrawItem = VerDrawTime
    OnClick = VerTimePicked
    OnChange = VerTimePicked
    OnDblClick = VerTimeOpen
  END CREATE
  CREATE VerWhatBtn AS RButton
    Caption = TR$("What &Changed?")
    Hint = TR$("What this version changed (Git calls this a diff)")
    ShowHint = 1
    OnClick = VerWhatClick
  END CREATE
  CREATE VerBackBtn AS RButton
    Caption = TR$("&Go Back...")
    Hint = TR$("Go back to this version: a new version with its files (Git calls this a revert)")
    ShowHint = 1
    OnClick = VerBackClick
  END CREATE
  CREATE VerUndoBtn AS RButton
    Caption = TR$("&Undo")
    ShowHint = 1
    OnClick = VerUndoClick
  END CREATE
  CREATE VerUndoNote AS RLabel
    WordWrap = 1: AutoSize = 0
  END CREATE
END CREATE

' ---- What Changed? / Local History: a document --------------------------------------------
CREATE VerDiffPane AS RPanel
  BevelOuter = 0
  AccessibleName = TR$("What changed")
  CREATE VerDiffTop AS RPanel
    Align = 1
    Height = 52
    BevelOuter = 0
    CREATE VerDiffHead AS RLabel
      Left = 12: Top = 9: Width = 400
    END CREATE
    CREATE VerDiffSub AS RLabel
      Left = 12: Top = 27: Width = 400
    END CREATE
    CREATE VerDiffAct AS RButton
      Caption = TR$("&Restore This Copy")
      OnClick = VerDiffActClick
    END CREATE
  END CREATE
  CREATE VerDiffList AS RListBox
    AccessibleName = TR$("Files")
    OnClick = VerDiffPicked
    OnChange = VerDiffPicked
  END CREATE
  CREATE VerDiff AS RDiffView
    ReadOnly = 1
    Mode = "inline"
    AccessibleName = TR$("What changed")
  END CREATE
END CREATE

' ---- Save a Version... ---------------------------------------------------------------------
CREATE VerSaveDialog AS RForm
  Caption = TR$("Save a Version")
  Width = 560: Height = 460
  BorderStyle = 3
  CREATE VerSaveTitle AS RLabel
    Caption = TR$("Save a Version")
  END CREATE
  CREATE VerSaveLead AS RLabel
    WordWrap = 1: AutoSize = 0
    Caption = TR$("A version keeps the files you choose as they are now, with a few words about what changed, so you can always come back to them. Git calls it a commit.")
  END CREATE
  CREATE VerMsgLabel AS RLabel
    Caption = TR$("&What changed:")
  END CREATE
  CREATE VerMsgEdit AS REdit
    Placeholder = TR$("Say in a few words what this version changes")
    AccessibleName = TR$("What changed")
    OnChange = VerMsgChanged
  END CREATE
  CREATE VerFilesLabel AS RLabel
    Caption = TR$("&Files in this version:")
  END CREATE
  CREATE VerFilesList AS RListView
    ViewStyle = vsReport
    CheckBoxes = 1
    RowSelect = 1
    ReadOnly = 1
    AccessibleName = TR$("Files in this version: Space ticks or unticks one")
  END CREATE
  CREATE VerNameLabel AS RLabel
    Caption = TR$("&Your name:")
  END CREATE
  CREATE VerNameEdit AS REdit
    AccessibleName = TR$("Your name, saved with the version")
  END CREATE
  ' (the check for keys and passwords: a card, its mark in the warning
  ' colour, what was found in words, the choice to save anyway)
  CREATE VerWarn AS RPanel
    BevelOuter = 0
    Visible = 0
    CREATE VerWarnMark AS RPanel
      Left = 0: Top = 0: Width = 4
      BevelOuter = 0
    END CREATE
    CREATE VerWarnIcon AS RImage
      Width = 16: Height = 16
      AccessibleHidden = 1
    END CREATE
    CREATE VerWarnText AS RLabel
      WordWrap = 1: AutoSize = 0
    END CREATE
    CREATE VerWarnLines AS RLabel
      WordWrap = 1: AutoSize = 0
    END CREATE
    CREATE VerAnyway AS RCheckBox
      Caption = TR$("These aren't real keys or passwords: save them &anyway")
      OnClick = VerAnywayClick
    END CREATE
  END CREATE
  CREATE VerSaveNo AS RButton
    Caption = TR$("Cancel")
    Cancel = 1
    OnClick = VerSaveNoClick
  END CREATE
  CREATE VerSaveOk AS RButton
    Caption = TR$("&Save Version")
    Default = 1
    OnClick = VerSaveOkClick
  END CREATE
END CREATE

' ---- setting up, the look ---------------------------------------------------------------------

SUB SetUpVersions
  DIM k AS INTEGER
  IF Reg.ValueExists("VersionsName") THEN VerAuthor = Reg.ReadString("VersionsName")
  VerPanel.OnResize = VerLayOut
  VerDiffPane.OnResize = VerDiffLayOut
  ' (a tab beside Project, hidden until it's wanted — View > Versions, a
  ' versions command, a project that has versions —: the side pane keeps
  ' room for Project and Search's titles)
  Dock.AddPane(VerPanel, TR$("Versions"), "tab:projecttree", "versions")
  Dock.HidePane("verpanel")
  Dock.ShowPane("projecttree")
  VerBackItem(0).Caption = TR$("The &Whole Project")
  VerBackItem(1).Caption = TR$("Only This &File")
  FOR k = 0 TO 1
    VerBackItem(k).Tag = k
    VerBackItem(k).OnClick = VerBackPicked
    VerBackMenu.AddItems VerBackItem(k)
  NEXT
  VerFilesList.AddColumns TR$("File"), TR$("Change")
  VerRefresh
END SUB

SUB ApplyVersionsChrome
  DIM rows AS INTEGER
  IF ThemeName = "classic" THEN rows = 16 ELSE rows = 22
  VerPanel.Color = TC("page")
  VerTop.Color = TC("page")
  VerHead.Font = UIFont
  VerHead.Font.Bold = 1
  VerHead.Font.Color = TC("page.text")
  VerSub.Font = DimFont
  VerSub.Font.Color = TC("page.dim")
  VerEmptyText.Font = UIFont
  VerEmptyText.Font.Color = TC("page.text")
  VerLocalNote.Font = DimFont
  VerLocalNote.Font.Color = TC("page.dim")
  VerUndoNote.Font = DimFont
  VerUndoNote.Font.Color = TC("page.dim")
  VerChangesHead.Font = DimFont
  VerChangesHead.Font.Color = TC("page.dim")
  VerTimeHead.Font = DimFont
  VerTimeHead.Font.Color = TC("page.dim")
  VerChanges.Font = UIFont
  VerChanges.ItemHeight = rows
  VerTimeline.Font = UIFont
  VerTimeline.ItemHeight = rows * 2
  VerSaveBtn.Font = UIFont
  VerStartBtn.Font = UIFont
  VerLocalBtn.Font = UIFont
  VerWhatBtn.Font = UIFont
  VerBackBtn.Font = UIFont
  VerUndoBtn.Font = UIFont
  ' (the What Changed document)
  VerDiffPane.Color = TC("page")
  VerDiffTop.Color = TC("page")
  VerDiffHead.Font = UIFont
  VerDiffHead.Font.Bold = 1
  VerDiffHead.Font.Color = TC("page.text")
  VerDiffSub.Font = DimFont
  VerDiffSub.Font.Color = TC("page.dim")
  VerDiffList.Font = UIFont
  VerDiffList.ItemHeight = rows
  VerDiffAct.Font = UIFont
  ' (Save a Version: the chrome font on the form, its title bigger)
  VerSaveDialog.Font.Name = UIFont.Name: VerSaveDialog.Font.Size = UIFont.Size
  VerSaveTitle.Font = SubFont
  VerSaveTitle.Font.Color = TC("page.text")
  VerSaveLead.Font.Color = TC("page.dim")
  VerWarn.Color = TC("card")
  VerWarnMark.Color = TC("status.warning.border")
  VerWarnText.Font = UIFont
  VerWarnText.Font.Bold = 1
  VerWarnText.Font.Color = TC("page.text")
  VerWarnLines.Font = MonoFont
  VerWarnLines.Font.Color = TC("page.text")
  VerBmp(0).LoadIcon "warning", 16
  VerWarnIcon.BMPHandle = VerBmp(0)
  VerLayOut
  VerDiffLayOut
  VerSaveLayOut
  VerChanges.Repaint
  VerTimeline.Repaint
END SUB

' ---- the state: refreshed after a save, an action, a project opened ---------------------------

' (Path relative to the project's folder, `/` separated)
FUNCTION VerRel(Path AS STRING) AS STRING
  DIM p AS STRING, f AS STRING
  p = REPLACESUBSTR$(Path, "\", "/")
  f = REPLACESUBSTR$(Proj.Folder, "\", "/")
  IF f <> "" AND LCASE$(LEFT$(p, LEN(f) + 1)) = LCASE$(f + "/") THEN p = MID$(p, LEN(f) + 2)
  Result = p
END FUNCTION

' (a time as a person says it: "just now", "5 minutes ago", "yesterday")
FUNCTION VerAgo(Seconds AS LONG) AS STRING
  DIM d AS LONG
  d = Repo.Now - Seconds
  IF d < 60 THEN
    Result = TR$("just now")
  ELSEIF d < 3600 THEN
    Result = TRN$("{n} minute ago", "{n} minutes ago", d \ 60)
  ELSEIF d < 86400 THEN
    Result = TRN$("{n} hour ago", "{n} hours ago", d \ 3600)
  ELSEIF d < 172800 THEN
    Result = TR$("yesterday")
  ELSE
    Result = TRN$("{n} day ago", "{n} days ago", d \ 86400)
  END IF
END FUNCTION

' The project's versions read again: the pane, the badges, the status bar,
' the editors' change marks.
SUB VerRefresh
  DIM t AS STRING, n AS INTEGER, i AS INTEGER, row AS STRING, d AS INTEGER, c AS STRING
  IF ProjectOpen THEN
    IF Repo.Folder <> Proj.Folder THEN Repo.Folder = Proj.Folder ELSE Repo.Refresh
  ELSE
    Repo.Folder = ""
  END IF
  IF VerAuthor <> "" THEN Repo.Author = VerAuthor
  ' (the changes)
  VerChanges.Clear
  VerChCount = 0
  c = Repo.Changes
  n = TALLY(c, CHR$(10)) + 1
  IF c = "" THEN n = 0
  FOR i = 1 TO n
    IF VerChCount <= VERMAX THEN
      row = FIELD$(c, CHR$(10), i)
      VerChLetter(VerChCount) = FIELD$(row, CHR$(9), 1)
      VerChPath(VerChCount) = FIELD$(row, CHR$(9), 2)
      ' (what a screen reader says of the row: the file and the change in words)
      SELECT CASE VerChLetter(VerChCount)
        CASE "N": VerChanges.AddItems VerChPath(VerChCount) + ", " + TR$("new, not in a version yet")
        CASE "D": VerChanges.AddItems VerChPath(VerChCount) + ", " + TR$("removed since the last version")
        CASE ELSE: VerChanges.AddItems VerChPath(VerChCount) + ", " + TR$("changed since the last version")
      END SELECT
      VerChCount = VerChCount + 1
    END IF
  NEXT
  ' (the timeline)
  VerTimeline.Clear
  VerTimeCount = 0
  IF Repo.Versioned THEN
    t = Repo.Timeline(VERMAX)
    n = TALLY(t, CHR$(10)) + 1
    IF t = "" THEN n = 0
    FOR i = 1 TO n
      row = FIELD$(t, CHR$(10), i)
      VerTimeId(VerTimeCount) = FIELD$(row, CHR$(9), 1)
      VerTimeWhen(VerTimeCount) = VAL(FIELD$(row, CHR$(9), 3))
      VerTimeWho(VerTimeCount) = FIELD$(row, CHR$(9), 4)
      VerTimeMsg(VerTimeCount) = FIELD$(row, CHR$(9), 5)
      VerTimeKind(VerTimeCount) = FIELD$(row, CHR$(9), 6)
      IF VerTimeKind(VerTimeCount) = "checkpoint" THEN
        VerTimeline.AddItems TR$("AI checkpoint: {what}, {when}", "what", VerTimeMsg(VerTimeCount), "when", VerAgo(VerTimeWhen(VerTimeCount)))
      ELSE
        VerTimeline.AddItems VerTimeMsg(VerTimeCount) + ", " + TR$("saved by {who}, {when}", "who", VerTimeWho(VerTimeCount), "when", VerAgo(VerTimeWhen(VerTimeCount)))
      END IF
      VerTimeCount = VerTimeCount + 1
    NEXT
    IF VerTimeCount > 0 THEN VerTimeline.ItemIndex = 0
  END IF
  ' (the Project pane's badges: the old ones off, the new ones on)
  n = TALLY(VerBadged, CHR$(10))
  FOR i = 1 TO n
    ProjectTree.FileBadge FIELD$(VerBadged, CHR$(10), i), ""
  NEXT
  VerBadged = ""
  FOR i = 0 TO VerChCount - 1
    ProjectTree.FileBadge VerChPath(i), VerChLetter(i)
    VerBadged = VerBadged + VerChPath(i) + CHR$(10)
  NEXT
  ' (the editors' marks against the last version)
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) THEN VerApplyBase d
  NEXT
  VerShowState
  LayOutStatus
  UpdateCommands
END SUB

' Document d's gutter marks: against its file in the last version when the
' project has versions (a file the last version hadn't: every line added),
' else since its last save.
SUB VerApplyBase(d AS INTEGER)
  DIM rel AS STRING
  IF d < 0 OR d >= DocCount THEN EXIT SUB
  IF DocOpen(d) = 0 THEN EXIT SUB
  rel = VerRel(DocFile(d))
  IF Repo.Versioned = 0 OR VerTimeCount = 0 OR LEFT$(rel, 1) = "/" OR MID$(rel, 2, 1) = ":" THEN
    IF CodeDoc(d).ChangesSince <> "save" THEN CodeDoc(d).ChangesSince = "save"
    EXIT SUB
  END IF
  IF Repo.HasFile(rel, "HEAD") THEN
    CodeDoc(d).ChangeBase = Repo.FileAt(rel, "HEAD")
  ELSE
    CodeDoc(d).ChangeBase = ""
  END IF
END SUB

' The pane's words and which of its parts show.
SUB VerShowState
  DIM vis AS INTEGER, k AS INTEGER, u AS STRING
  vis = Flag(Repo.Versioned <> 0)
  VerSaveBtn.Visible = vis
  VerChangesHead.Visible = vis
  VerChanges.Visible = vis
  VerTimeHead.Visible = vis
  VerTimeline.Visible = vis
  VerWhatBtn.Visible = vis
  VerBackBtn.Visible = vis
  VerUndoBtn.Visible = vis
  VerUndoNote.Visible = vis
  VerEmptyText.Visible = Flag(vis = 0)
  VerStartBtn.Visible = Flag(vis = 0 AND Repo.Available <> 0)
  VerLocalNote.Visible = Flag(vis = 0)
  VerLocalBtn.Visible = 1
  IF ProjectOpen = 0 THEN
    VerSub.Caption = TR$("Open a project to see its versions")
    VerEmptyText.Caption = TR$("Versions keep your project's files as they were at each step, so nothing is ever lost.")
    VerStartBtn.Visible = 0
    VerLocalBtn.Visible = 0
    VerLocalNote.Caption = ""
  ELSEIF Repo.Available = 0 THEN
    VerSub.Caption = TR$("Not in a browser")
    VerEmptyText.Caption = TR$("Versions need RapidR Studio on your computer: a browser can't keep them yet.")
    VerLocalNote.Caption = TR$("Studio still keeps a copy of each file every time you save it, in this browser. Local History shows them.")
  ELSEIF vis = 0 THEN
    VerSub.Caption = TR$("No versions yet")
    VerEmptyText.Caption = TR$("A version keeps all your files as they are now, with a few words about what changed, so you can always go back. Git calls versions commits.")
    VerLocalNote.Caption = TR$("Until then Studio keeps a copy of each file every time you save it. Local History shows them.")
  ELSE
    ' (short: the pane is narrow — the status bar says it whole)
    IF VerChCount = 0 THEN
      VerSub.Caption = TR$("{experiment} · all saved", "experiment", Repo.Experiment)
    ELSE
      VerSub.Caption = TRN$("{experiment} · {n} change", "{experiment} · {n} changes", VerChCount, "experiment", Repo.Experiment)
    END IF
    VerSub.Hint = TR$("{experiment} is the experiment you're on (Git calls it a branch)", "experiment", Repo.Experiment)
    VerSub.ShowHint = 1
  END IF
  IF VerChCount = 0 THEN
    VerChangesHead.Caption = TR$("No changes since the last version")
  ELSE
    VerChangesHead.Caption = TR$("Changes not saved as a version ({n})", "n", Num(VerChCount))
  END IF
  u = Repo.CanUndo
  VerUndoBtn.Enabled = Flag(u <> "")
  IF u = "" THEN
    VerUndoNote.Caption = ""
    VerUndoBtn.Hint = TR$("Nothing to undo")
  ELSE
    VerUndoNote.Caption = TR$("Undo: {what}", "what", u)
    VerUndoBtn.Hint = TR$("Undo: {what}", "what", u)
  END IF
  VerUndoBtn.AccessibleDescription = VerUndoBtn.Hint
  k = VerTimeline.ItemIndex
  VerWhatBtn.Enabled = Flag(k >= 0 AND k < VerTimeCount)
  VerBackBtn.Enabled = Flag(k >= 0 AND k < VerTimeCount)
  VerSaveBtn.Enabled = Flag(VerChCount > 0 AND Repo.ReadOnly = 0)
  VerLayOut
END SUB

' ("1 change not saved as a version", "No changes since the last version")
FUNCTION VerChangesWords AS STRING
  IF VerChCount = 0 THEN
    Result = TR$("no changes since the last version")
  ELSE
    Result = TRN$("{n} change not saved as a version", "{n} changes not saved as a version", VerChCount)
  END IF
END FUNCTION

' (chrome.inc's LayOutStatus: what the project's panel says after its name —
' the experiment and the changes —, "" without versions)
FUNCTION VerStatusText AS STRING
  IF ProjectOpen = 0 OR Repo.Versioned = 0 THEN EXIT FUNCTION
  Result = "  ·  " + Repo.Experiment + " · " + VerChangesWords
END FUNCTION

' The pane's controls for its size: the header; then, without versions, the
' words and Start; with them, the changes (up to five rows), the timeline
' (the rest), the buttons under it and Undo's words.
SUB VerLayOut
  DIM w AS INTEGER, h AS INTEGER, ch AS INTEGER, hh AS INTEGER, y AS INTEGER, x AS INTEGER, bw AS INTEGER, th AS INTEGER, lh AS INTEGER, rows AS INTEGER
  ch = UiCompactH
  hh = UiPaneHeadH
  w = VerPanel.ClientWidth
  h = VerPanel.ClientHeight
  IF w < 120 THEN w = 260
  IF h < 120 THEN h = 500
  IF VerTop.Height <> hh THEN VerTop.Height = hh
  ' (the header: the title and the experiment's line, the pane's width —
  ' a narrow side pane: Save a Version... goes under the changes it saves)
  x = w - 12
  th = VerHead.Height + 2 + VerSub.Height
  VerHead.Left = 12: VerHead.Top = (hh - th) \ 2: VerHead.Width = x - 12
  VerSub.Left = 12: VerSub.Top = VerHead.Top + VerHead.Height + 2: VerSub.Width = x - 12
  y = hh + 12
  IF Repo.Versioned = 0 THEN
    lh = AISetLinesHigh(VerEmptyText.Caption, w - 24)
    VerEmptyText.Left = 12: VerEmptyText.Top = y: VerEmptyText.Width = w - 24: VerEmptyText.Height = lh
    y = y + lh + 12
    IF VerStartBtn.Visible THEN
      bw = 230
      IF bw > w - 24 THEN bw = w - 24
      VerStartBtn.Left = 12: VerStartBtn.Top = y: VerStartBtn.Width = bw: VerStartBtn.Height = ch
      y = y + ch + 16
    END IF
    lh = AISetLinesHigh(VerLocalNote.Caption, w - 24) + 4
    VerLocalNote.Left = 12: VerLocalNote.Top = y: VerLocalNote.Width = w - 24: VerLocalNote.Height = lh
    y = y + lh + 8
    VerLocalBtn.Left = 12: VerLocalBtn.Top = y: VerLocalBtn.Width = 132: VerLocalBtn.Height = ch
    EXIT SUB
  END IF
  VerLocalBtn.Visible = 0
  ' (the changes: up to five rows; none — a line saying so; Save a
  ' Version... under them, the pane's main action)
  ' (a longer language wraps the line: "No hay cambios desde la última versión")
  lh = AISetLinesHigh(VerChangesHead.Caption, w - 24)
  IF lh < 18 THEN lh = 18
  VerChangesHead.Left = 12: VerChangesHead.Top = y: VerChangesHead.Width = w - 24: VerChangesHead.Height = lh
  y = y + lh + 2
  rows = VerChCount
  IF rows > 5 THEN rows = 5
  IF rows = 0 THEN
    VerChanges.Visible = 0
  ELSE
    VerChanges.Left = 8: VerChanges.Top = y: VerChanges.Width = w - 16: VerChanges.Height = rows * VerChanges.ItemHeight + 4
    y = y + VerChanges.Height + 8
  END IF
  bw = 140
  IF bw > w - 16 THEN bw = w - 16
  VerSaveBtn.Left = 8: VerSaveBtn.Top = y: VerSaveBtn.Width = bw: VerSaveBtn.Height = ch
  y = y + ch + 16
  VerTimeHead.Left = 12: VerTimeHead.Top = y: VerTimeHead.Width = w - 24: VerTimeHead.Height = 18
  y = y + 20
  ' (What Changed?, Go Back..., Undo: compact, 8 apart; a narrow pane puts
  ' Undo on a row of its own — the timeline takes what's left above them
  ' and Undo's words)
  lh = 1
  IF 8 + 112 + 8 + 92 + 8 + 72 > w - 8 THEN lh = 2
  th = h - y - (8 + lh * (ch + 8) + 34)
  IF th < VerTimeline.ItemHeight * 2 + 4 THEN th = VerTimeline.ItemHeight * 2 + 4
  VerTimeline.Left = 8: VerTimeline.Top = y: VerTimeline.Width = w - 16: VerTimeline.Height = th
  y = y + th + 8
  x = 8
  VerWhatBtn.Left = x: VerWhatBtn.Top = y: VerWhatBtn.Width = 112: VerWhatBtn.Height = ch
  x = x + 112 + 8
  VerBackBtn.Left = x: VerBackBtn.Top = y: VerBackBtn.Width = 92: VerBackBtn.Height = ch
  x = x + 92 + 8
  VerUndoBtn.Left = x: VerUndoBtn.Top = y: VerUndoBtn.Width = 72: VerUndoBtn.Height = ch
  IF lh = 2 THEN
    y = y + ch + 8
    VerUndoBtn.Left = 8: VerUndoBtn.Top = y
    ' (Undo's words beside it)
    VerUndoNote.Left = 88: VerUndoNote.Top = y: VerUndoNote.Width = w - 96: VerUndoNote.Height = 32
  ELSE
    y = y + ch + 6
    VerUndoNote.Left = 12: VerUndoNote.Top = y: VerUndoNote.Width = w - 24: VerUndoNote.Height = 32
  END IF
END SUB

' (Text as it fits Room pixels of the lists' font: whole, or cut with "…")
FUNCTION VerFit(Text AS STRING, Room AS INTEGER) AS STRING
  DIM k AS INTEGER
  VerMeasure.Font.Name = UIFont.Name: VerMeasure.Font.Size = UIFont.Size
  IF VerMeasure.TextWidth(Text) <= Room OR LEN(Text) < 4 THEN
    Result = Text
    EXIT FUNCTION
  END IF
  FOR k = LEN(Text) - 1 TO 1 STEP -1
    IF VerMeasure.TextWidth(LEFT$(Text, k) + "…") <= Room THEN EXIT FOR
  NEXT
  Result = LEFT$(Text, k) + "…"
END FUNCTION

' A timeline row: a ring on the line of versions (an AI's checkpoint a
' smaller, lighter one), the message, then who and when — dimmed for a
' checkpoint; the selected row in the selection's colours.
SUB VerDrawTime(Index AS INTEGER, State AS INTEGER, R AS RRect)
  DIM bg AS LONG, fg AS LONG, dm AS LONG, lc AS LONG, cx AS INTEGER, cy AS INTEGER, rr AS INTEGER, half AS INTEGER, sel AS INTEGER, room AS INTEGER
  IF Index < 0 OR Index >= VerTimeCount THEN EXIT SUB
  sel = Flag(Index = VerTimeline.ItemIndex)
  IF sel THEN
    bg = TC("selected"): fg = TC("selected.text"): dm = TC("selected.text")
  ELSE
    bg = TC("window"): fg = TC("text"): dm = TC("page.dim")
  END IF
  lc = TC("border")
  VerTimeline.FillRect R.Left, R.Top, R.Right, R.Bottom, bg
  half = (R.Bottom - R.Top) \ 2
  cx = R.Left + 16: cy = R.Top + half \ 2 + 4
  ' (the line through the rings: from the row above, to the row below)
  IF Index > 0 THEN VerTimeline.Line cx, R.Top, cx, cy, lc
  IF Index < VerTimeCount - 1 THEN VerTimeline.Line cx, cy, cx, R.Bottom, lc
  room = R.Right - R.Left - 36
  IF VerTimeKind(Index) = "checkpoint" THEN
    rr = 3
    VerTimeline.Circle cx - rr, cy - rr, cx + rr + 1, cy + rr + 1, dm, bg
    VerTimeline.TextOut R.Left + 32, R.Top + 3, VerFit(TR$("AI checkpoint: {what}", "what", VerTimeMsg(Index)), room), dm, -1
    VerTimeline.TextOut R.Left + 32, R.Top + half + 1, VerFit(VerAgo(VerTimeWhen(Index)), room), dm, -1
  ELSE
    rr = 5
    VerTimeline.Circle cx - rr, cy - rr, cx + rr + 1, cy + rr + 1, fg, bg
    VerTimeline.TextOut R.Left + 32, R.Top + 3, VerFit(VerTimeMsg(Index), room), fg, -1
    VerTimeline.TextOut R.Left + 32, R.Top + half + 1, VerFit(TR$("{who} · {when}", "who", VerTimeWho(Index), "when", VerAgo(VerTimeWhen(Index))), room), dm, -1
  END IF
END SUB

' A changed file's row: its letter in a soft status square (M changed, N
' new, D removed), then its path.
SUB VerDrawChange(Index AS INTEGER, State AS INTEGER, R AS RRect)
  DIM bg AS LONG, fg AS LONG, k AS STRING, sx AS INTEGER, sy AS INTEGER, sel AS INTEGER
  IF Index < 0 OR Index >= VerChCount THEN EXIT SUB
  sel = Flag(Index = VerChanges.ItemIndex)
  IF sel THEN
    bg = TC("selected"): fg = TC("selected.text")
  ELSE
    bg = TC("window"): fg = TC("text")
  END IF
  VerChanges.FillRect R.Left, R.Top, R.Right, R.Bottom, bg
  SELECT CASE VerChLetter(Index)
    CASE "N": k = "status.success"
    CASE "D": k = "status.danger"
    CASE ELSE: k = "status.info"
  END SELECT
  sx = R.Left + 8: sy = R.Top + (R.Bottom - R.Top - 16) \ 2
  VerChanges.RoundRect sx, sy, sx + 16, sy + 16, 6, 6, TC(k + ".border")
  VerChanges.RoundRect sx + 1, sy + 1, sx + 15, sy + 15, 5, 5, TC(k)
  VerChanges.TextOut sx + 4, sy + 1, VerChLetter(Index), TC(k + ".text"), -1
  VerChanges.TextOut R.Left + 32, R.Top + (R.Bottom - R.Top - 16) \ 2, VerFit(VerChPath(Index), R.Right - R.Left - 36), fg, -1
END SUB

SUB VerTimePicked
  VerShowState
  VerTimeline.Repaint
END SUB

' (a double click, Enter: what it changed)
SUB VerTimeOpen
  VerShowDiff "version"
END SUB

SUB VerChangePicked
  VerChanges.Repaint
END SUB

' (a changed file opened: its document)
SUB VerChangeOpen
  DIM k AS INTEGER
  k = VerChanges.ItemIndex
  IF k < 0 OR k >= VerChCount THEN EXIT SUB
  IF VerChLetter(k) <> "D" THEN OpenDocument Proj.Folder + "/" + VerChPath(k)
END SUB

' ---- the actions ------------------------------------------------------------------------------

' View > Versions: the pane, to the front, the keyboard in it.
SUB VerShowPane
  IF Dock.PaneState("verpanel") = "" THEN Dock.AddPane(VerPanel, TR$("Versions"), "tab:projecttree", "versions")
  Dock.FocusPane("verpanel")
  VerRefresh
END SUB

SUB VerStartClick
  RunCommand "versions.start"
END SUB

SUB VerSaveClick
  RunCommand "versions.save"
END SUB

SUB VerWhatClick
  RunCommand "versions.whatChanged"
END SUB

SUB VerUndoClick
  RunCommand "versions.undo"
END SUB

SUB VerLocalClick
  RunCommand "versions.localHistory"
END SUB

SUB VerBackClick
  VerBackItem(1).Enabled = Flag(ActiveCode >= 0)
  IF ActiveCode >= 0 THEN VerBackItem(1).Caption = TR$("Only {file}", "file", FileTitle(DocFile(ActiveCode))) ELSE VerBackItem(1).Caption = TR$("Only This &File")
  VerBackMenu.Popup Screen.MouseX, Screen.MouseY
END SUB

SUB VerBackPicked(Sender AS RMenuItem)
  IF Sender.Tag = 0 THEN RunCommand "versions.goBack" ELSE RunCommand "versions.goBackFile"
END SUB

' Start Versioning This Project: the project's folder keeps versions from
' now on (what Git calls git init, with RapidR's .gitignore).
SUB VerStart
  IF ProjectOpen = 0 THEN EXIT SUB
  IF Repo.Start = 0 THEN
    OutputLine TR$("Versions didn't start: {why}", "why", Repo.Error)
    SetState "stopped", TR$("Versions didn't start")
    VerShowPane
    EXIT SUB
  END IF
  OutputLine TR$("This project keeps versions now: {folder}", "folder", Proj.Folder)
  SetState "stopped", TR$("Versions started: save your first version")
  Application.Announce TR$("Versions started. Save a Version keeps your files as they are now.")
  VerRefresh
  Dock.ShowPane("verpanel")
END SUB

' (the files open with changes: saved first — a version keeps what's on
' the disk —, asked as when closing; 0 when the person cancelled)
FUNCTION VerSaveOpenFiles AS INTEGER
  Result = AskToSaveAll
END FUNCTION

' Save a Version...: the dialog, filled.
SUB ShowSaveVersion
  DIM i AS INTEGER
  IF ProjectOpen = 0 THEN EXIT SUB
  IF Repo.Versioned = 0 THEN
    VerStart
    IF Repo.Versioned = 0 THEN EXIT SUB
  END IF
  IF VerSaveOpenFiles = 0 THEN EXIT SUB
  VerRefresh
  IF VerChCount = 0 THEN
    SetState "stopped", TR$("Nothing changed since the last version")
    Application.Announce TR$("Nothing changed since the last version, so there is nothing to save.")
    EXIT SUB
  END IF
  VerMsgEdit.Text = ""
  IF VerAuthor = "" THEN VerAuthor = Repo.Author
  IF VerAuthor = "" THEN VerAuthor = ENVIRON$("USER")
  VerNameEdit.Text = VerAuthor
  VerFilesList.Clear
  FOR i = 0 TO VerChCount - 1
    VerFilesList.AddItems VerChPath(i)
    SELECT CASE VerChLetter(i)
      CASE "N": VerFilesList.AddSubItem i, TR$("new")
      CASE "D": VerFilesList.AddSubItem i, TR$("removed")
      CASE ELSE: VerFilesList.AddSubItem i, TR$("changed")
    END SELECT
    VerFilesList.Item(i).Checked = 1
  NEXT
  VerSaveFound = 0
  VerWarn.Visible = 0
  VerAnyway.Checked = 0
  VerSaveOk.Caption = TR$("&Save Version")
  VerSaveLayOut
  VerMsgChanged
  VerSaveDialog.Center
  VerMsgEdit.SetFocus
  VerSaveDialog.ShowModal
END SUB

' (the files ticked, a line each)
FUNCTION VerTickedFiles AS STRING
  DIM i AS INTEGER, s AS STRING
  FOR i = 0 TO VerFilesList.ItemCount - 1
    IF VerFilesList.Item(i).Checked THEN s = s + VerChPath(i) + CHR$(10)
  NEXT
  Result = s
END FUNCTION

SUB VerMsgChanged
  VerSaveOk.Enabled = Flag(LTRIM$(RTRIM$(VerMsgEdit.Text)) <> "" AND (VerSaveFound = 0 OR VerAnyway.Checked <> 0))
END SUB

SUB VerAnywayClick
  IF VerAnyway.Checked THEN VerSaveOk.Caption = TR$("Save &Anyway") ELSE VerSaveOk.Caption = TR$("&Save Version")
  VerMsgChanged
END SUB

SUB VerSaveNoClick
  VerSaveDialog.Close
END SUB

' Save Version: the check for keys and passwords first (said in the dialog,
' saved only once the person ticks "save anyway"), then the version.
SUB VerSaveOkClick
  DIM fl AS STRING, id AS STRING, s AS STRING, n AS INTEGER, i AS INTEGER, row AS STRING
  fl = VerTickedFiles
  IF fl = "" THEN
    VerWarnShow TR$("Tick at least one file to save in this version."), ""
    EXIT SUB
  END IF
  VerAuthor = LTRIM$(RTRIM$(VerNameEdit.Text))
  Repo.Author = VerAuthor
  Reg.OpenKey(STUDIO_KEY, 1)
  Reg.WriteString("VersionsName", VerAuthor)
  id = Repo.SaveVersion(VerMsgEdit.Text, fl, Flag(VerSaveFound <> 0 AND VerAnyway.Checked <> 0))
  IF id = "" THEN
    IF Repo.ErrorKind = "secrets" THEN
      s = ""
      n = TALLY(Repo.Secrets, CHR$(10)) + 1
      FOR i = 1 TO n
        IF i <= 4 THEN
          row = FIELD$(Repo.Secrets, CHR$(10), i)
          s = s + TR$("{file}, line {line}: {what}", "file", FIELD$(row, CHR$(9), 1), "line", FIELD$(row, CHR$(9), 2), "what", FIELD$(row, CHR$(9), 3)) + CHR$(10) + "  " + FIELD$(row, CHR$(9), 4) + CHR$(10)
        END IF
      NEXT
      IF n > 4 THEN s = s + TRN$("and {n} more line", "and {n} more lines", n - 4)
      VerSaveFound = 1
      VerWarnShow TRN$("This line looks like a key or a password. Versions are often shared, and a secret saved in one stays in the timeline: take it out first, or save anyway if it isn't a real one.", "These {n} lines look like keys or passwords. Versions are often shared, and a secret saved in one stays in the timeline: take them out first, or save anyway if they aren't real ones.", n), s
      Application.Announce TR$("Not saved: lines look like keys or passwords. The dialog lists them.")
    ELSE
      VerWarnShow Repo.Error, ""
    END IF
    EXIT SUB
  END IF
  VerSaveDialog.Close
  OutputLine TR$("Saved the version “{message}” ({id})", "message", VerMsgEdit.Text, "id", LEFT$(id, 7))
  SetState "stopped", TR$("Saved the version “{message}”", "message", VerMsgEdit.Text)
  Application.Announce TR$("Version saved.")
  VerRefresh
END SUB

' The dialog's warning strip: Text in bold, Lines under it in the code's
' font; "save anyway" only for keys and passwords.
SUB VerWarnShow(Text AS STRING, Lines AS STRING)
  VerWarnText.Caption = Text
  VerWarnLines.Caption = Lines
  VerAnyway.Visible = Flag(VerSaveFound <> 0)
  VerWarn.AccessibleName = Text + " " + Lines
  VerWarn.Visible = 1
  VerSaveLayOut
  VerMsgChanged
END SUB

' The dialog for its contents (docs/design-guidelines.md: 20 around, 24-
' pixel fields, 32-pixel buttons, the labels 4 above their fields).
SUB VerSaveLayOut
  DIM pad AS INTEGER, w AS INTEGER, y AS INTEGER, fh AS INTEGER, bh AS INTEGER, bw AS INTEGER, lh AS INTEGER, wh AS INTEGER
  pad = 20: w = 520
  fh = UiCompactH: bh = UiButtonH
  IF ThemeName = "classic" THEN bw = 84 ELSE bw = 112
  y = pad
  VerSaveTitle.Left = pad: VerSaveTitle.Top = y: VerSaveTitle.Width = w: VerSaveTitle.Height = 24
  y = y + 28
  lh = AISetLinesHigh(VerSaveLead.Caption, w) + 2
  VerSaveLead.Left = pad: VerSaveLead.Top = y: VerSaveLead.Width = w: VerSaveLead.Height = lh
  y = y + lh + 16
  VerMsgLabel.Left = pad: VerMsgLabel.Top = y: VerMsgLabel.Width = w: VerMsgLabel.Height = 18
  y = y + 22
  VerMsgEdit.Left = pad: VerMsgEdit.Top = y: VerMsgEdit.Width = w: VerMsgEdit.Height = fh
  y = y + fh + 16
  VerFilesLabel.Left = pad: VerFilesLabel.Top = y: VerFilesLabel.Width = w: VerFilesLabel.Height = 18
  y = y + 22
  VerFilesList.Left = pad: VerFilesList.Top = y: VerFilesList.Width = w: VerFilesList.Height = 120
  VerFilesList.Column(0).Width = w - 140
  VerFilesList.Column(1).Width = 110
  y = y + 120 + 16
  VerNameLabel.Left = pad: VerNameLabel.Top = y + (fh - 18) \ 2: VerNameLabel.Width = 96: VerNameLabel.Height = 18
  VerNameEdit.Left = pad + 100: VerNameEdit.Top = y: VerNameEdit.Width = 240: VerNameEdit.Height = fh
  y = y + fh + 16
  IF VerWarn.Visible THEN
    lh = AISetLinesHigh(VerWarnText.Caption, (w - 44) * 9 \ 10)
    IF lh < 18 THEN lh = 18
    VerWarnIcon.Left = 16: VerWarnIcon.Top = 12
    VerWarnText.Left = 40: VerWarnText.Top = 10: VerWarnText.Width = w - 52: VerWarnText.Height = lh
    wh = 10 + lh + 6
    IF VerWarnLines.Caption <> "" THEN
      lh = (TALLY(VerWarnLines.Caption, CHR$(10)) + 1) * 18
      VerWarnLines.Left = 40: VerWarnLines.Top = wh: VerWarnLines.Width = w - 52: VerWarnLines.Height = lh
      VerWarnLines.Visible = 1
      wh = wh + lh + 4
    ELSE
      VerWarnLines.Visible = 0
    END IF
    IF VerAnyway.Visible THEN
      VerAnyway.Left = 40: VerAnyway.Top = wh: VerAnyway.Width = w - 52: VerAnyway.Height = 20
      wh = wh + 24
    END IF
    wh = wh + 8
    VerWarn.Left = pad: VerWarn.Top = y: VerWarn.Width = w: VerWarn.Height = wh
    VerWarnMark.Height = wh
    y = y + wh + 16
  END IF
  VerSaveOk.Width = bw + 16: VerSaveOk.Height = bh: VerSaveOk.Left = pad + w - bw - 16: VerSaveOk.Top = y
  VerSaveNo.Width = bw: VerSaveNo.Height = bh: VerSaveNo.Left = VerSaveOk.Left - 8 - bw: VerSaveNo.Top = y
  VerSaveDialog.ClientWidth = w + 2 * pad
  VerSaveDialog.ClientHeight = y + bh + pad
END SUB

' (the timeline row picked: its index, -1 when none)
FUNCTION VerPicked AS INTEGER
  Result = VerTimeline.ItemIndex
  IF Result >= VerTimeCount THEN Result = -1
END FUNCTION

' Go Back to This Version: asked first (a small confirmation: what happens,
' Cancel the default), then a new version with that version's files — the
' whole project, or (OneFile) the file open. The documents follow.
SUB VerGoBack(OneFile AS INTEGER)
  DIM k AS INTEGER, id AS STRING, rel AS STRING, q AS STRING, msg AS STRING
  k = VerPicked
  IF k < 0 THEN
    SetState "stopped", TR$("Pick a version in the timeline first")
    EXIT SUB
  END IF
  msg = VerTimeMsg(k)
  IF OneFile THEN
    IF ActiveCode < 0 THEN EXIT SUB
    rel = VerRel(DocFile(ActiveCode))
    q = TR$("Go back to “{message}” for {file}?", "message", msg, "file", FileTitle(rel))
    ConsentBegin q, TR$("{file} becomes as it was in that version, saved as a new version. The other files stay as they are, and the newer versions stay on the timeline. Undo takes it back.", "file", FileTitle(rel))
  ELSE
    q = TR$("Go back to “{message}”?", "message", msg)
    ConsentBegin q, TR$("Your files become as they were in that version, saved as a new version. The newer versions stay on the timeline, and Undo takes it back. Git calls this a revert.")
  END IF
  IF ConsentAsk("", "", TR$("&Go Back"), TR$("Cancel")) = 0 THEN EXIT SUB
  IF VerSaveOpenFiles = 0 THEN EXIT SUB
  IF OneFile THEN id = Repo.GoBackFile(rel, VerTimeId(k)) ELSE id = Repo.GoBack(VerTimeId(k))
  IF id = "" THEN
    OutputLine TR$("Didn't go back: {why}", "why", Repo.Error)
    SetState "error", TR$("Didn't go back: see the Output")
    Application.Announce Repo.Error
    VerRefresh
    EXIT SUB
  END IF
  VerReloadDocs
  OutputLine TR$("Went back to “{message}”: a new version ({id}). Undo takes it back.", "message", msg, "id", LEFT$(id, 7))
  SetState "stopped", TR$("Went back to “{message}”", "message", msg)
  Application.Announce TR$("Went back to {message}. Undo takes it back.", "message", msg)
  VerRefresh
END SUB

' Undo: the last action on the versions taken back (said in words), the
' documents following.
SUB VerUndo
  DIM w AS STRING
  IF Repo.CanUndo = "" THEN
    SetState "stopped", TR$("Nothing to undo in the versions")
    EXIT SUB
  END IF
  IF VerSaveOpenFiles = 0 THEN EXIT SUB
  w = Repo.Undo
  IF w = "" THEN
    OutputLine TR$("Couldn't undo: {why}", "why", Repo.Error)
    SetState "error", TR$("Couldn't undo: see the Output")
    Application.Announce Repo.Error
    VerRefresh
    EXIT SUB
  END IF
  VerReloadDocs
  OutputLine TR$("Undid: {what}", "what", w)
  SetState "stopped", TR$("Undid: {what}", "what", w)
  Application.Announce TR$("Undid: {what}", "what", w)
  VerRefresh
END SUB

' The open documents whose files the versions rewrote, read again (none has
' unsaved changes: they were saved first). Their text before is in the
' local history.
SUB VerReloadDocs
  DIM d AS INTEGER, disk AS STRING
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) AND DocDirty(d) = 0 THEN
      disk = Repo.FileAt(DocFile(d), "")
      IF disk <> CodeDoc(d).Text AND FILEEXISTS(DocFile(d)) THEN
        Loading = 1
        CodeDoc(d).LoadFromFile DocFile(d)
        Loading = 0
        DocDirty(d) = 0
        DesignRead d
        DocTitles d
        ProjectFileChanged d
      END IF
      ' (what the versions wrote is Studio's own: no change from outside —
      ' external.inc)
      ExtOwnWrite DocFile(d)
    END IF
  NEXT
  AnalyzeActive
END SUB

' ---- What Changed? / Local History ------------------------------------------------------------

' The document: "version" — what the timeline's picked version changed;
' "changes" — what changed since the last version; "history" — the copies
' the local history kept of the file open, each against the editor's text.
SUB VerShowDiff(Mode AS STRING)
  DIM t AS STRING, n AS INTEGER, i AS INTEGER, row AS STRING, k AS INTEGER, title AS STRING
  VerDiffMode = Mode
  VerDiffList.Clear
  VerDiffRowCount = 0
  IF Mode = "history" THEN
    IF ActiveCode < 0 THEN
      SetState "stopped", TR$("Open a file to see its local history")
      EXIT SUB
    END IF
    VerDiffFile = DocFile(ActiveCode)
    IF Repo.Folder = "" THEN Repo.Folder = Proj.Folder
    t = Repo.LocalHistory(VerDiffFile)
    title = TR$("Local History: {file}", "file", FileTitle(VerDiffFile))
    VerDiffHead.Caption = title
    VerDiffSub.Caption = TR$("A copy each time the file was saved, kept in the project's .rapidr folder (never in a version)")
    VerDiffList.AccessibleName = TR$("Copies of {file}, newest first", "file", FileTitle(VerDiffFile))
    VerDiffAct.Caption = TR$("&Restore This Copy")
    VerDiffAct.Hint = TR$("Puts this copy's text in the editor; Undo (Ctrl+Z) takes it back")
    VerDiffAct.Visible = 1
    n = TALLY(t, CHR$(10)) + 1
    IF t = "" THEN n = 0
    FOR i = 1 TO n
      row = FIELD$(t, CHR$(10), i)
      IF VerDiffRowCount <= VERMAX THEN
        VerDiffRow(VerDiffRowCount) = FIELD$(row, CHR$(9), 1)
        SELECT CASE FIELD$(row, CHR$(9), 3)
          CASE "on-disk": VerDiffList.AddItems TR$("As it was on disk, {when}", "when", VerAgo(VAL(FIELD$(row, CHR$(9), 2))))
          CASE "before-reload": VerDiffList.AddItems TR$("Before reloading, {when}", "when", VerAgo(VAL(FIELD$(row, CHR$(9), 2))))
          CASE "before-restore": VerDiffList.AddItems TR$("Before a restore, {when}", "when", VerAgo(VAL(FIELD$(row, CHR$(9), 2))))
          CASE "deleted-outside": VerDiffList.AddItems TR$("When it was deleted outside Studio, {when}", "when", VerAgo(VAL(FIELD$(row, CHR$(9), 2))))
          CASE ELSE: VerDiffList.AddItems TR$("Saved, {when}", "when", VerAgo(VAL(FIELD$(row, CHR$(9), 2))))
        END SELECT
        VerDiffRowCount = VerDiffRowCount + 1
      END IF
    NEXT
    IF n = 0 THEN VerDiffSub.Caption = TR$("No copies yet: Studio keeps one each time you save this file.")
  ELSEIF Mode = "disk" THEN
    ' (a file changed outside Studio while it had unsaved changes here: the
    ' disk's version beside the editor's — external.inc; VerDiffFile set)
    title = TR$("Compare: {file}", "file", FileTitle(VerDiffFile))
    VerDiffHead.Caption = TR$("{file}: the disk's version and yours", "file", FileTitle(VerDiffFile))
    VerDiffSub.Caption = TR$("Left: on disk, changed outside Studio. Right: yours, not saved.")
    VerDiffList.AccessibleName = TR$("Files")
    VerDiffAct.Caption = TR$("&Use Disk Version")
    VerDiffAct.Hint = TR$("Loads the file as it is on disk. Your version is kept in the local history, and Undo brings it back.")
    VerDiffAct.Visible = 1
    VerDiffRow(0) = VerDiffFile
    VerDiffList.AddItems FileTitle(VerDiffFile)
    VerDiffRowCount = 1
  ELSE
    IF Mode = "version" THEN
      k = VerPicked
      IF k < 0 THEN
        SetState "stopped", TR$("Pick a version in the timeline first")
        EXIT SUB
      END IF
      VerDiffVersion = VerTimeId(k)
      t = Repo.VersionFiles(VerDiffVersion)
      title = TR$("What Changed: {message}", "message", VerTimeMsg(k))
      VerDiffHead.Caption = TR$("What changed in “{message}”", "message", VerTimeMsg(k))
      VerDiffSub.Caption = TR$("{who} · {when} · {files}", "who", VerTimeWho(k), "when", VerAgo(VerTimeWhen(k)), "files", TRN$("{n} file", "{n} files", TALLY(t, CHR$(10)) + Flag(t <> "")))
    ELSE
      VerDiffVersion = ""
      t = Repo.VersionFiles("")
      title = TR$("What Changed")
      VerDiffHead.Caption = TR$("What changed since the last version")
      VerDiffSub.Caption = VerChangesWords
    END IF
    VerDiffList.AccessibleName = TR$("Files that changed")
    VerDiffAct.Visible = 0
    n = TALLY(t, CHR$(10)) + 1
    IF t = "" THEN n = 0
    FOR i = 1 TO n
      row = FIELD$(t, CHR$(10), i)
      IF VerDiffRowCount <= VERMAX THEN
        VerDiffRow(VerDiffRowCount) = FIELD$(row, CHR$(9), 2)
        SELECT CASE FIELD$(row, CHR$(9), 1)
          CASE "N": VerDiffList.AddItems TR$("{file} (new)", "file", VerDiffRow(VerDiffRowCount))
          CASE "D": VerDiffList.AddItems TR$("{file} (removed)", "file", VerDiffRow(VerDiffRowCount))
          CASE ELSE: VerDiffList.AddItems VerDiffRow(VerDiffRowCount)
        END SELECT
        VerDiffRowCount = VerDiffRowCount + 1
      END IF
    NEXT
  END IF
  IF Dock.PaneState("verdiffpane") = "" THEN Dock.AddPane(VerDiffPane, title, "documents", "compare")
  Dock.PaneTitle("verdiffpane", title)
  Dock.FocusPane("verdiffpane")
  VerDiffLayOut
  IF VerDiffRowCount > 0 THEN
    VerDiffList.ItemIndex = 0
    VerDiffPicked
  ELSE
    VerDiff.LeftText = ""
    VerDiff.RightText = ""
  END IF
  VerDiffList.SetFocus
END SUB

' (the diff's text in the editor's plain ink, not coloured as code: the
' changed words' marks are tinted, and the code's colours on them don't
' all read at 4.5:1 yet — every word stays readable)
FUNCTION VerDiffLanguage(Path AS STRING) AS STRING
  Result = "plain.txt"
END FUNCTION

' A row picked: its file's change, or the copy against the editor's text.
SUB VerDiffPicked
  DIM k AS INTEGER, p AS STRING, ext AS STRING, d AS INTEGER
  k = VerDiffList.ItemIndex
  IF k < 0 OR k >= VerDiffRowCount THEN EXIT SUB
  IF VerDiffMode = "disk" THEN
    p = VerDiffFile
    VerDiff.Kind = "text"
    VerDiff.Language = VerDiffLanguage(p)
    IF FILEEXISTS(p) THEN VerDiff.LeftText = Proj.FileText(p) ELSE VerDiff.LeftText = ""
    d = DocOf(p)
    IF d >= 0 THEN VerDiff.RightText = CodeDoc(d).Text
    ' (the first difference in view)
    IF VerDiff.HunkCount > 0 THEN VerDiff.CurrentHunk = 0
    EXIT SUB
  END IF
  IF VerDiffMode = "history" THEN
    p = VerDiffFile
    VerDiff.Kind = "text"
    VerDiff.Language = VerDiffLanguage(p)
    VerDiff.LeftText = Repo.SnapshotText(VerDiffRow(k))
    d = DocOf(p)
    IF d >= 0 THEN VerDiff.RightText = CodeDoc(d).Text ELSE VerDiff.RightText = Repo.FileAt(p, "")
    EXIT SUB
  END IF
  p = VerDiffRow(k)
  ext = FileExt(p)
  IF ext = "csv" THEN VerDiff.Kind = "table" ELSE VerDiff.Kind = "text"
  VerDiff.Language = VerDiffLanguage(p)
  IF VerDiffMode = "version" THEN
    VerDiff.LeftText = Repo.FileAt(p, VerDiffVersion + "^")
    VerDiff.RightText = Repo.FileAt(p, VerDiffVersion)
  ELSE
    VerDiff.LeftText = Repo.FileAt(p, "HEAD")
    VerDiff.RightText = Repo.FileAt(p, "")
  END IF
END SUB

' Restore This Copy: the copy's text in the editor as one undo step (the
' editor's text kept in the local history first); saving it is the
' person's choice.
SUB VerDiffActClick
  DIM k AS INTEGER, d AS INTEGER, txt AS STRING, patch AS STRING
  ' (Compare's Use Disk Version: external.inc)
  IF VerDiffMode = "disk" THEN
    d = DocOf(VerDiffFile)
    IF d >= 0 THEN
      ExtUseDisk d
      VerDiffPicked
    END IF
    EXIT SUB
  END IF
  IF VerDiffMode <> "history" THEN EXIT SUB
  k = VerDiffList.ItemIndex
  IF k < 0 OR k >= VerDiffRowCount THEN EXIT SUB
  txt = Repo.SnapshotText(VerDiffRow(k))
  IF txt = "" AND Repo.Error <> "" THEN
    SetState "error", Repo.Error
    EXIT SUB
  END IF
  d = DocOf(VerDiffFile)
  IF d < 0 THEN
    OpenDocument VerDiffFile
    d = DocOf(VerDiffFile)
  END IF
  IF d < 0 THEN EXIT SUB
  Repo.Snapshot VerDiffFile, "before-restore"
  patch = AITools.Patch(CodeDoc(d).Text, txt)
  IF patch <> "" THEN
    CodeDoc(d).ApplyPatches patch
  END IF
  ' (the comparison again: the copy and the editor now the same)
  VerDiffPicked
  Dock.FocusPane(DocPane(d))
  SetState "stopped", TR$("Restored a copy of {file}: save it to keep it, Ctrl+Z takes it back", "file", FileTitle(VerDiffFile))
  Application.Announce TR$("Restored. Save the file to keep it; Undo takes it back.")
END SUB

SUB VerDiffLayOut
  DIM w AS INTEGER, h AS INTEGER, ch AS INTEGER, hh AS INTEGER, bw AS INTEGER, lw AS INTEGER, x AS INTEGER, th AS INTEGER
  ch = UiCompactH
  hh = UiPaneHeadH
  w = VerDiffPane.ClientWidth
  h = VerDiffPane.ClientHeight
  IF w < 200 THEN w = 800
  IF h < 120 THEN h = 500
  IF VerDiffTop.Height <> hh THEN VerDiffTop.Height = hh
  bw = 150
  VerDiffAct.Left = w - 12 - bw: VerDiffAct.Top = (hh - ch) \ 2: VerDiffAct.Width = bw: VerDiffAct.Height = ch
  IF VerDiffAct.Visible THEN x = VerDiffAct.Left - 12 ELSE x = w - 12
  th = VerDiffHead.Height + 2 + VerDiffSub.Height
  VerDiffHead.Left = 12: VerDiffHead.Top = (hh - th) \ 2: VerDiffHead.Width = x - 12
  VerDiffSub.Left = 12: VerDiffSub.Top = VerDiffHead.Top + VerDiffHead.Height + 2: VerDiffSub.Width = x - 12
  lw = 220
  IF lw > w \ 3 THEN lw = w \ 3
  VerDiffList.Left = 0: VerDiffList.Top = hh: VerDiffList.Width = lw: VerDiffList.Height = h - hh
  VerDiff.Left = lw + 1: VerDiff.Top = hh: VerDiff.Width = w - lw - 1: VerDiff.Height = h - hh
END SUB

' ---- Studio's hooks ------------------------------------------------------------------------------

' (documents.inc's SaveDocument: before the file is written — what the disk
' had, kept in the local history when it isn't already)
SUB VerBeforeSave(d AS INTEGER)
  IF ProjectOpen = 0 THEN EXIT SUB
  IF Repo.Folder = "" THEN Repo.Folder = Proj.Folder
  ' (a new file the disk hasn't yet: nothing kept)
  Repo.Snapshot DocFile(d), "on-disk"
END SUB

' (after it was written: a copy of what was saved; the versions' state again)
SUB VerAfterSave(d AS INTEGER)
  IF ProjectOpen = 0 THEN EXIT SUB
  IF Repo.Folder = "" THEN Repo.Folder = Proj.Folder
  IF Repo.Snapshot(DocFile(d), "saved") = "" AND Repo.Error <> "" THEN OutputLine TR$("Local history: {why}", "why", Repo.Error)
  VerRefresh
END SUB

' (project.inc: a project opened or closed — one with versions shows the
' Versions tab beside Project, Project staying in front)
SUB VersionsProjectChanged
  VerBadged = ""
  ProjectTree.ClearBadges
  VerRefresh
  IF Repo.Versioned AND Dock.PaneVisible("verpanel") = 0 THEN
    Dock.ShowPane("verpanel")
    Dock.ShowPane("projecttree")
  END IF
END SUB

' The commands (commands.inc's table): 1 when Id was one of them.
FUNCTION VersionsCommand(Id AS STRING) AS INTEGER
  DIM d AS INTEGER, l AS INTEGER
  Result = 1
  SELECT CASE Id
    CASE "versions.pane": VerShowPane
    CASE "versions.start": VerStart
    CASE "versions.save": ShowSaveVersion
    CASE "versions.whatChanged"
      IF Repo.Versioned = 0 THEN
        VerShowPane
      ELSEIF VerPicked >= 0 THEN
        VerShowDiff "version"
      ELSE
        VerShowDiff "changes"
      END IF
    CASE "versions.changes": VerShowDiff "changes"
    CASE "versions.goBack": VerGoBack 0
    CASE "versions.goBackFile": VerGoBack 1
    CASE "versions.undo": VerUndo
    CASE "versions.localHistory": VerShowDiff "history"
    CASE "versions.showChange", "versions.undoChange"
      ' (the change at the caret's line: shown under its lines, or undone)
      d = ActiveCode
      IF d < 0 THEN EXIT FUNCTION
      l = CodeDoc(d).CaretLine
      IF CodeDoc(d).ChangeAt(l) = "" THEN
        IF CodeDoc(d).ChangesSince = "version" THEN SetState "stopped", TR$("This line is as the last version has it") ELSE SetState "stopped", TR$("Start versioning the project to see what changed since a version")
        EXIT FUNCTION
      END IF
      IF Id = "versions.showChange" THEN
        CodeDoc(d).ShowChange l
        Application.Announce TR$("Line {n} was: {text}", "n", Num(l), "text", CodeDoc(d).ChangeText(l))
      ELSE
        CodeDoc(d).RevertChange l
        Application.Announce TR$("The change was undone. Ctrl+Z brings it back.")
      END IF
    CASE ELSE: Result = 0
  END SELECT
END FUNCTION

' (a test step: versions:<what>=<value> — the flows drive the pane through it)
SUB VersionsStep(S AS STRING)
  DIM k AS INTEGER, what AS STRING, v AS STRING, i AS INTEGER
  k = INSTR(S, "=")
  IF k > 0 THEN
    what = LEFT$(S, k - 1): v = MID$(S, k + 1)
  ELSE
    what = S
  END IF
  SELECT CASE what
    CASE "save"
      ' (versions:save=<message>: every changed file, as Save Version does)
      IF Repo.Versioned = 0 THEN VerStart
      IF Repo.SaveVersion(v, "", 0) = "" THEN OutputLine "versions:save: " + Repo.Error
      VerRefresh
    CASE "pick"
      ' (versions:pick=<message>: that version selected in the timeline)
      FOR i = 0 TO VerTimeCount - 1
        IF VerTimeMsg(i) = v THEN VerTimeline.ItemIndex = i
      NEXT
      VerTimePicked
    CASE "goback"
      ' (versions:goback: the picked version, the question answered Go Back)
      i = VerPicked
      IF i >= 0 THEN
        IF Repo.GoBack(VerTimeId(i)) = "" THEN OutputLine "versions:goback: " + Repo.Error
        VerReloadDocs
        VerRefresh
      END IF
    CASE "author": VerAuthor = v: Repo.Author = v
    CASE "checkpoint"
      ' (versions:checkpoint=<label>: an AI's checkpoint, as the assistant
      ' keeps one before it changes files — RAI's own wiring is lane G8)
      IF Repo.Checkpoint(v) = "" THEN OutputLine "versions:checkpoint: " + Repo.Error
      VerRefresh
    CASE "report"
      ' (versions:report=<file>: what the pane, the Project pane's badge of
      ' that file and the status bar say)
      OutputLine "[versions] versioned=" + Num(Flag(Repo.Versioned <> 0)) + " changes=" + Num(VerChCount) + " timeline=" + Num(VerTimeCount) + " experiment=" + Repo.Experiment + " badge=" + ProjectTree.FileBadge(v) + " status=" + Status.Panel(1).Caption
  END SELECT
END SUB

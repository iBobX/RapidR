' versions.inc: Versions (docs/versions-plan.md, lanes G3 and
' VERSIONS-POLISH) — the project's versions in plain words, each word with
' its Git name in a tooltip, and nothing ever lost:
'   - the Versions pane (a tab beside Project and Search in every layout,
'     an old layout's too, unless the person closed it; View > Versions):
'     a header with the project, its experiment and its changes as chips;
'     no versions yet: what they are and Start Keeping Versions; then the
'     files not saved as a version, by folder (closed folders, a filter
'     past 20 files), Save a Version... (the pane's primary button), the
'     Timeline (a line through a dot for each version, the current one
'     filled, an AI's checkpoints lighter), What Changed?, Go Back...,
'     Undo and what it takes back;
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
DECLARE SUB VerFillChanges
DECLARE SUB VerToggle(Row AS INTEGER, WantOpen AS INTEGER)
DECLARE SUB VerFilterChanged
DECLARE SUB VerChangeKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE SUB VerChangeMouse(Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB VerChipLayOut(Chip AS RPanel, Ground AS RCanvas, Icon AS RImage, Words AS RLabel, X AS INTEGER, Y AS INTEGER)
DECLARE FUNCTION VerTextW(Text AS STRING, Style AS INTEGER) AS INTEGER
DECLARE FUNCTION VerLineH(Dimmed AS INTEGER) AS INTEGER
DECLARE FUNCTION VerWrap(Text AS STRING, Room AS INTEGER, MaxLines AS INTEGER, Style AS INTEGER) AS STRING
DECLARE FUNCTION VerLinesOf(Text AS STRING, Room AS INTEGER, MaxLines AS INTEGER, Style AS INTEGER) AS INTEGER
DECLARE FUNCTION VerBtnW(Caption AS STRING) AS INTEGER
DECLARE FUNCTION VerFit(Text AS STRING, Room AS INTEGER) AS STRING
DECLARE FUNCTION VerFitMiddle(Text AS STRING, Room AS INTEGER) AS STRING
DECLARE FUNCTION VerAgoShort(Seconds AS LONG) AS STRING
DECLARE FUNCTION VerDirOf(Path AS STRING) AS STRING
DECLARE FUNCTION VerDirShut(K AS INTEGER) AS INTEGER
DECLARE FUNCTION VerRowKey(Row AS INTEGER) AS STRING
DECLARE FUNCTION VerStateWords(Letter AS STRING) AS STRING

CONST VERMAX = 200

' (Repo, the RRepository: decl.inc)
' (the timeline as last read: each row's id, kind, message, who, when)
DIM VerTimeId(VERMAX) AS STRING
DIM VerTimeKind(VERMAX) AS STRING
DIM VerTimeMsg(VERMAX) AS STRING
DIM VerTimeWho(VERMAX) AS STRING
DIM VerTimeWhen(VERMAX) AS LONG
DIM VerTimeCount AS INTEGER
' (the changes not saved as a version: each one's letter and path, and
' its folder's number in the list — -1 when the filter leaves it out)
CONST VERCHMAX = 3000
' (more changes than this: the filter box)
CONST VERFILTERAT = 20
DIM VerChLetter(VERCHMAX) AS STRING
DIM VerChPath(VERCHMAX) AS STRING
DIM VerChDir(VERCHMAX) AS INTEGER
DIM VerChCount AS INTEGER
' (the folders with changes — "" the project's own —, how many each, in
' the list's order)
DIM VerDir(VERCHMAX) AS STRING
DIM VerDirFiles(VERCHMAX) AS INTEGER
DIM VerDirOrder(VERCHMAX) AS INTEGER
DIM VerDirCount AS INTEGER
' (the list's rows: a folder's (Ref its number) or a file's (Ref the change's))
DIM VerRowFolder(VERCHMAX) AS INTEGER
DIM VerRowRef(VERCHMAX) AS INTEGER
DIM VerRowCount AS INTEGER
' (the folders the person closed or opened, a line each, while Studio
' runs; a filter typed: every folder open)
DIM VerShut AS STRING
DIM VerOpened AS STRING
DIM VerFiltering AS INTEGER
' (the Versions tab closed by the person: it stays closed — Settings'
' "VersionsTab")
DIM VerTabClosed AS INTEGER
' (Studio itself placing the tab — at start, a layout read —: not the
' person's choice; the dock's OnPaneChange runs right after each call)
DIM VerQuiet AS INTEGER
' (Undo's words wanted under the buttons: shown when the pane has room)
DIM VerUndoWanted AS INTEGER
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
DIM VerBmp(4) AS RBitmap
' (the lists' text measured: a long message cut with "…")
DIM VerMeasure AS RBitmap
' (the header's second line, whole: cut to fit, its tooltip says it all)
DIM VerSubFull AS STRING
' (a test's look at the pane: the layout report's problems — VersionsStep)
DIM VerEyes AS RWindowCapture

' ---- the Versions pane ------------------------------------------------------------------
' (docs/design-guidelines.md "Pane anatomy": a header — the title, the
' project's name under it, the experiment's and the changes' chips —; then
' the changes, grouped by folder, with Save a Version... the pane's one
' primary button; the timeline, a line through a dot for each version;
' What Changed?, Go Back... and Undo, and what Undo takes back in words)
CREATE VerPanel AS RPanel
  BevelOuter = 0
  AccessibleName = TR$("Versions")
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
    ' (the experiment you're on: a soft info chip, the branch icon)
    CREATE VerBranch AS RPanel
      BevelOuter = 0
      ShowHint = 1
      CREATE VerBranchChip AS RCanvas
        Left = 0: Top = 0
        Tag = 0
        OnPaint = ChipPaint
      END CREATE
      CREATE VerBranchIcon AS RImage
        Width = 16: Height = 16
      END CREATE
      CREATE VerBranchText AS RLabel
        Transparent = 1
      END CREATE
    END CREATE
    ' (the changes not saved as a version, or all saved: info / success)
    CREATE VerCount AS RPanel
      BevelOuter = 0
      CREATE VerCountChip AS RCanvas
        Left = 0: Top = 0
        Tag = 0
        OnPaint = ChipPaint
      END CREATE
      CREATE VerCountIcon AS RImage
        Width = 16: Height = 16
      END CREATE
      CREATE VerCountText AS RLabel
        Transparent = 1
      END CREATE
    END CREATE
  END CREATE
  ' (no project, or one without versions: what they are, one button)
  CREATE VerEmptyIcon AS RImage
    Width = 24: Height = 24
    AccessibleHidden = 1
  END CREATE
  CREATE VerEmptyTitle AS RLabel
    WordWrap = 1: AutoSize = 0
  END CREATE
  CREATE VerEmptyText AS RLabel
    WordWrap = 1: AutoSize = 0
  END CREATE
  CREATE VerStartBtn AS RButton
    Caption = TR$("&Start Keeping Versions of This Project")
    Primary = 1
    Hint = TR$("Start keeping versions of this project (Git calls this git init)")
    ShowHint = 1
    AccessibleName = TR$("Start keeping versions of this project")
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
    Caption = TR$("Changes")
  END CREATE
  CREATE VerSaveBtn AS RButton
    Caption = TR$("&Save a Version...")
    Primary = 1
    Hint = TR$("Save a version of your files (Git calls this a commit)")
    ShowHint = 1
    AccessibleDescription = TR$("Keeps the changed files as they are now, with a few words about what changed (Git calls this a commit)")
    OnClick = VerSaveClick
  END CREATE
  CREATE VerNoChanges AS RLabel
    WordWrap = 1: AutoSize = 0
  END CREATE
  CREATE VerFilter AS REdit
    Placeholder = TR$("Filter the changed files")
    AccessibleName = TR$("Filter the changed files")
    AccessibleDescription = TR$("Shows only the files whose name or folder has these letters")
    OnChange = VerFilterChanged
  END CREATE
  CREATE VerChanges AS RListBox
    Style = lbOwnerDrawFixed
    AccessibleName = TR$("Files not saved as a version, by folder")
    AccessibleDescription = TR$("A double click or Enter opens a file; Right opens a folder, Left closes it")
    OnDrawItem = VerDrawChange
    OnClick = VerChangePicked
    OnChange = VerChangePicked
    OnDblClick = VerChangeOpen
    OnKeyDown = VerChangeKey
    OnMouseUp = VerChangeMouse
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
    ShowHint = 1
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
  ' (Studio's settings: the name versions are saved with; the Versions tab
  ' closed on purpose by the person — remembered —, it stays closed; View >
  ' Versions brings it back)
  Reg.RootKey = HKCU
  IF Reg.KeyExists(STUDIO_KEY) THEN
    Reg.OpenKey(STUDIO_KEY, 0)
    IF Reg.ValueExists("VersionsName") THEN VerAuthor = Reg.ReadString("VersionsName")
    IF Reg.ValueExists("VersionsTab") THEN VerTabClosed = Flag(Reg.ReadString("VersionsTab") = "closed")
    Reg.CloseKey
  END IF
  VerPanel.OnResize = VerLayOut
  VerDiffPane.OnResize = VerDiffLayOut
  ' (a tab beside Project and Search in every layout, Project in front)
  VerQuiet = 1
  Dock.AddPane(VerPanel, TR$("Versions"), "tab:projecttree", "versions")
  IF VerTabClosed THEN Dock.HidePane("verpanel")
  Dock.ShowPane("projecttree")
  VerQuiet = 0
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

' (a layout read — the one saved last, an old Studio's without Versions —:
' the Versions tab back beside Project unless the person closed it; the
' tab in front stays in front)
SUB VerLayoutLoaded
  DIM front AS STRING, where AS STRING
  VerQuiet = 1
  ' (closed on purpose: closed, whatever the layout says)
  IF VerTabClosed THEN
    IF Dock.PaneState("verpanel") <> "hidden" THEN
      IF Dock.PaneVisible("verpanel") THEN front = "projecttree"
      Dock.HidePane("verpanel")
      IF front <> "" THEN Dock.ShowPane(front)
    END IF
  ELSEIF Dock.PaneState("verpanel") = "hidden" THEN
    IF Dock.PaneVisible("searchpane") THEN front = "searchpane" ELSE front = "projecttree"
    IF Dock.PaneState("searchpane") = "hidden" OR Dock.PaneState("searchpane") = "" THEN where = "tab:projecttree" ELSE where = "tab:searchpane"
    IF Dock.PaneState("projecttree") = "hidden" AND where = "tab:projecttree" THEN
      Dock.ShowPane("verpanel")
    ELSE
      Dock.DockPane("verpanel", where)
      Dock.ShowPane(front)
    END IF
  END IF
  VerQuiet = 0
END SUB

' (window.inc's PaneChanged: the Versions tab closed by the person — its
' close button — is remembered, so it isn't put back at the next start)
SUB VerPaneChanged(Name AS STRING)
  IF LCASE$(Name) <> "verpanel" OR VerQuiet THEN EXIT SUB
  IF Dock.PaneState("verpanel") = "hidden" THEN
    IF VerTabClosed THEN EXIT SUB
    VerTabClosed = 1
  ELSE
    IF VerTabClosed = 0 THEN EXIT SUB
    VerTabClosed = 0
  END IF
  Reg.RootKey = HKCU
  Reg.OpenKey(STUDIO_KEY, 1)
  IF VerTabClosed THEN Reg.WriteString("VersionsTab", "closed") ELSE Reg.WriteString("VersionsTab", "shown")
  Reg.CloseKey
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
  ' (the chips: a soft pill on the header's ground, the icon, the words
  ' in the status's strong colour; read as one by a screen reader)
  VerBranch.Color = TC("page")
  VerBranchChip.Color = TC("page")
  VerBranchChip.AccessibleHidden = 1
  VerBranchIcon.AccessibleHidden = 1
  VerBranchText.AccessibleHidden = 1
  VerBranchText.Font = UIFont
  VerBranchText.Font.Color = TC("status.info.text")
  VerBmp(1).LoadIcon "branch", 16
  VerBranchIcon.BMPHandle = VerBmp(1)
  VerCount.Color = TC("page")
  VerCountChip.Color = TC("page")
  VerCountChip.AccessibleHidden = 1
  VerCountIcon.AccessibleHidden = 1
  VerCountText.AccessibleHidden = 1
  VerCountText.Font = UIFont
  VerEmptyTitle.Font = UIFont
  VerEmptyTitle.Font.Bold = 1
  VerEmptyTitle.Font.Color = TC("page.text")
  VerEmptyText.Font = UIFont
  VerEmptyText.Font.Color = TC("page.dim")
  VerBmp(3).LoadIcon "timeline", 24
  VerEmptyIcon.BMPHandle = VerBmp(3)
  VerLocalNote.Font = DimFont
  VerLocalNote.Font.Color = TC("page.dim")
  VerUndoNote.Font = DimFont
  VerUndoNote.Font.Color = TC("page.dim")
  VerNoChanges.Font = UIFont
  VerNoChanges.Font.Color = TC("page.dim")
  VerChangesHead.Font = UIFont
  VerChangesHead.Font.Bold = 1
  VerChangesHead.Font.Color = TC("page.text")
  VerTimeHead.Font = UIFont
  VerTimeHead.Font.Bold = 1
  VerTimeHead.Font.Color = TC("page.text")
  VerFilter.Font = UIFont
  VerChanges.Font = UIFont
  VerChanges.ItemHeight = rows
  VerTimeline.Font = UIFont
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
  VerShowState
  VerDiffLayOut
  VerSaveLayOut
  VerBranchChip.Repaint
  VerCountChip.Repaint
  VerChanges.Repaint
  VerTimeline.Repaint
END SUB

' ---- measuring: the pane's words in their fonts ---------------------------------------------

' (Text's width in the chrome's font — Style 0 —, bold — 1 — or the
' secondary text's — 2)
FUNCTION VerTextW(Text AS STRING, Style AS INTEGER) AS INTEGER
  IF Style = 2 THEN
    VerMeasure.Font.Name = DimFont.Name: VerMeasure.Font.Size = DimFont.Size: VerMeasure.Font.Bold = 0
  ELSE
    VerMeasure.Font.Name = UIFont.Name: VerMeasure.Font.Size = UIFont.Size: VerMeasure.Font.Bold = Flag(Style = 1)
  END IF
  Result = VerMeasure.TextWidth(Text)
END FUNCTION

' (a line's height in the chrome's font, or the secondary one)
FUNCTION VerLineH(Dimmed AS INTEGER) AS INTEGER
  IF Dimmed THEN
    VerMeasure.Font.Name = DimFont.Name: VerMeasure.Font.Size = DimFont.Size
  ELSE
    VerMeasure.Font.Name = UIFont.Name: VerMeasure.Font.Size = UIFont.Size
  END IF
  VerMeasure.Font.Bold = 0
  Result = VerMeasure.TextHeight("Ag")
END FUNCTION

' Text wrapped at word breaks to Room pixels (in VerTextW's Style), at
' most MaxLines lines (0: all), the last cut with "…" when there is more:
' the lines, CHR$(10) between them.
FUNCTION VerWrap(Text AS STRING, Room AS INTEGER, MaxLines AS INTEGER, Style AS INTEGER) AS STRING
  DIM t AS STRING, cur AS STRING, wd AS STRING, out AS STRING, i AS INTEGER, j AS INTEGER, n AS INTEGER, lines AS INTEGER
  t = LTRIM$(RTRIM$(Text))
  IF t = "" THEN EXIT FUNCTION
  n = TALLY(t, " ") + 1
  lines = 1
  FOR i = 1 TO n
    wd = FIELD$(t, " ", i)
    IF cur = "" THEN
      cur = wd
    ELSEIF VerTextW(cur + " " + wd, Style) > Room THEN
      IF MaxLines > 0 AND lines >= MaxLines THEN
        ' (the last line: what's left of it, cut)
        FOR j = i TO n
          cur = cur + " " + FIELD$(t, " ", j)
        NEXT
        EXIT FOR
      END IF
      out = out + cur + CHR$(10)
      lines = lines + 1
      cur = wd
    ELSE
      cur = cur + " " + wd
    END IF
  NEXT
  ' (the last line cut with "…" when it doesn't fit)
  IF VerTextW(cur, Style) > Room AND LEN(cur) > 3 THEN
    FOR j = LEN(cur) - 1 TO 1 STEP -1
      IF VerTextW(LEFT$(cur, j) + "…", Style) <= Room THEN EXIT FOR
    NEXT
    cur = LEFT$(cur, j) + "…"
  END IF
  Result = out + cur
END FUNCTION

' (how many lines VerWrap makes)
FUNCTION VerLinesOf(Text AS STRING, Room AS INTEGER, MaxLines AS INTEGER, Style AS INTEGER) AS INTEGER
  DIM t AS STRING
  t = VerWrap(Text, Room, MaxLines, Style)
  IF t = "" THEN Result = 0 ELSE Result = TALLY(t, CHR$(10)) + 1
END FUNCTION

' (a button's width for its caption: 12 on each side, 72 at least)
FUNCTION VerBtnW(Caption AS STRING) AS INTEGER
  Result = VerTextW(REPLACESUBSTR$(Caption, "&", ""), 0) + 24
  IF Result < 72 THEN Result = 72
END FUNCTION

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
  DIM t AS STRING, n AS INTEGER, i AS INTEGER, row AS STRING, d AS INTEGER, c AS STRING, p AS LONG, q AS LONG
  IF ProjectOpen THEN
    IF Repo.Folder <> Proj.Folder THEN Repo.Folder = Proj.Folder ELSE Repo.Refresh
  ELSE
    Repo.Folder = ""
  END IF
  IF VerAuthor <> "" THEN Repo.Author = VerAuthor
  ' (the changes: a line each, walked once — thousands stay quick)
  VerChCount = 0
  c = Repo.Changes
  p = 1
  DO WHILE p <= LEN(c) AND VerChCount < VERCHMAX
    q = INSTR(p, c, CHR$(10))
    IF q = 0 THEN q = LEN(c) + 1
    row = MID$(c, p, q - p)
    IF row <> "" THEN
      VerChLetter(VerChCount) = FIELD$(row, CHR$(9), 1)
      VerChPath(VerChCount) = FIELD$(row, CHR$(9), 2)
      VerChCount = VerChCount + 1
    END IF
    p = q + 1
  LOOP
  VerFillChanges
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
      ' (what a screen reader says of the row: the words, who and when; the
      ' newest is the version the files are based on)
      IF VerTimeKind(VerTimeCount) = "checkpoint" THEN
        row = TR$("AI checkpoint: {what}, {when}", "what", VerTimeMsg(VerTimeCount), "when", VerAgo(VerTimeWhen(VerTimeCount)))
      ELSE
        row = VerTimeMsg(VerTimeCount) + ", " + TR$("saved by {who}, {when}", "who", VerTimeWho(VerTimeCount), "when", VerAgo(VerTimeWhen(VerTimeCount)))
      END IF
      IF VerTimeCount = 0 THEN row = row + ", " + TR$("the current version")
      VerTimeline.AddItems row
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

' ---- the changes, by folder --------------------------------------------------------------------

' (a path's folder: "" for the project's own)
FUNCTION VerDirOf(Path AS STRING) AS STRING
  DIM i AS INTEGER
  i = RINSTR(Path, "/")
  IF i > 0 THEN Result = LEFT$(Path, i - 1)
END FUNCTION

' (whether folder number K's rows are closed: as the person left it, else
' — a long list — a folder of more than five closed, so the list starts short)
FUNCTION VerDirShut(K AS INTEGER) AS INTEGER
  DIM key AS STRING
  IF VerDir(K) = "" OR VerFiltering THEN EXIT FUNCTION
  key = CHR$(10) + VerDir(K) + CHR$(10)
  IF INSTR(CHR$(10) + VerShut, key) > 0 THEN
    Result = 1
  ELSEIF INSTR(CHR$(10) + VerOpened, key) > 0 THEN
    Result = 0
  ELSE
    Result = Flag(VerChCount > VERFILTERAT AND VerDirFiles(K) > 5)
  END IF
END FUNCTION

' (what a row is, to find it again after the list is made again)
FUNCTION VerRowKey(Row AS INTEGER) AS STRING
  IF Row < 0 OR Row >= VerRowCount THEN EXIT FUNCTION
  IF VerRowFolder(Row) THEN Result = "d:" + VerDir(VerRowRef(Row)) ELSE Result = "f:" + VerChPath(VerRowRef(Row))
END FUNCTION

' (a change's state in words, for a screen reader)
FUNCTION VerStateWords(Letter AS STRING) AS STRING
  SELECT CASE Letter
    CASE "N": Result = TR$("new, not in a version yet")
    CASE "D": Result = TR$("deleted since the last version")
    CASE ELSE: Result = TR$("changed since the last version")
  END SELECT
END FUNCTION

' The changes list made again: the project's own files first, then each
' folder — its row (open or closed) and its files —, only the files the
' filter names; the row picked before picked again.
SUB VerFillChanges
  DIM i AS INTEGER, j AS INTEGER, k AS INTEGER, d AS STRING, f AS STRING, key AS STRING, shut AS INTEGER, s AS STRING, n AS INTEGER, pick AS INTEGER
  key = VerRowKey(VerChanges.ItemIndex)
  f = ""
  IF VerChCount > VERFILTERAT THEN f = LCASE$(LTRIM$(RTRIM$(VerFilter.Text)))
  VerFiltering = Flag(f <> "")
  ' (each change's folder, numbered as found; -1: the filter leaves it out)
  VerDirCount = 0
  FOR i = 0 TO VerChCount - 1
    VerChDir(i) = -1
    IF f = "" OR INSTR(LCASE$(VerChPath(i)), f) > 0 THEN
      d = VerDirOf(VerChPath(i))
      k = -1
      FOR j = VerDirCount - 1 TO 0 STEP -1
        IF VerDir(j) = d THEN
          k = j
          EXIT FOR
        END IF
      NEXT
      IF k < 0 THEN
        k = VerDirCount
        VerDir(k) = d
        VerDirFiles(k) = 0
        VerDirCount = VerDirCount + 1
      END IF
      VerDirFiles(k) = VerDirFiles(k) + 1
      VerChDir(i) = k
    END IF
  NEXT
  ' (the folders in order: the project's own first, then by name)
  FOR i = 0 TO VerDirCount - 1
    VerDirOrder(i) = i
  NEXT
  FOR i = 1 TO VerDirCount - 1
    k = VerDirOrder(i)
    j = i - 1
    DO WHILE j >= 0
      IF LCASE$(VerDir(VerDirOrder(j))) <= LCASE$(VerDir(k)) THEN EXIT DO
      VerDirOrder(j + 1) = VerDirOrder(j)
      j = j - 1
    LOOP
    VerDirOrder(j + 1) = k
  NEXT
  VerChanges.Clear
  VerRowCount = 0
  pick = -1
  FOR j = 0 TO VerDirCount - 1
    k = VerDirOrder(j)
    shut = VerDirShut(k)
    IF VerDir(k) <> "" AND VerRowCount < VERCHMAX THEN
      VerRowFolder(VerRowCount) = 1
      VerRowRef(VerRowCount) = k
      IF shut THEN s = TR$("closed") ELSE s = TR$("open")
      VerChanges.AddItems TR$("Folder {folder}, {changes}, {state}", "folder", VerDir(k), "changes", TRN$("{n} change", "{n} changes", VerDirFiles(k)), "state", s)
      IF key = "d:" + VerDir(k) THEN pick = VerRowCount
      VerRowCount = VerRowCount + 1
    END IF
    IF shut = 0 THEN
      FOR i = 0 TO VerChCount - 1
        IF VerChDir(i) = k AND VerRowCount < VERCHMAX THEN
          VerRowFolder(VerRowCount) = 0
          VerRowRef(VerRowCount) = i
          IF VerDir(k) = "" THEN
            VerChanges.AddItems TR$("{file}: {state}", "file", FileTitle(VerChPath(i)), "state", VerStateWords(VerChLetter(i)))
          ELSE
            VerChanges.AddItems TR$("{file} in {folder}: {state}", "file", FileTitle(VerChPath(i)), "folder", VerDir(k), "state", VerStateWords(VerChLetter(i)))
          END IF
          IF key = "f:" + VerChPath(i) THEN pick = VerRowCount
          VerRowCount = VerRowCount + 1
        END IF
      NEXT
    END IF
  NEXT
  IF pick >= 0 THEN VerChanges.ItemIndex = pick
END SUB

' Folder row Row opened or closed (WantOpen: 1 open, 0 closed, -1 the other
' way), remembered while Studio runs; the list and the pane again.
SUB VerToggle(Row AS INTEGER, WantOpen AS INTEGER)
  DIM d AS STRING, now AS INTEGER, want AS INTEGER
  IF Row < 0 OR Row >= VerRowCount THEN EXIT SUB
  IF VerRowFolder(Row) = 0 THEN EXIT SUB
  d = VerDir(VerRowRef(Row))
  now = Flag(VerDirShut(VerRowRef(Row)) = 0)
  IF WantOpen < 0 THEN want = Flag(now = 0) ELSE want = WantOpen
  IF want = now THEN EXIT SUB
  VerShut = REPLACESUBSTR$(CHR$(10) + VerShut, CHR$(10) + d + CHR$(10), CHR$(10))
  VerShut = MID$(VerShut, 2)
  VerOpened = REPLACESUBSTR$(CHR$(10) + VerOpened, CHR$(10) + d + CHR$(10), CHR$(10))
  VerOpened = MID$(VerOpened, 2)
  IF want THEN VerOpened = VerOpened + d + CHR$(10) ELSE VerShut = VerShut + d + CHR$(10)
  VerFillChanges
  VerLayOut
  IF want THEN Application.Announce TR$("{folder} opened", "folder", d) ELSE Application.Announce TR$("{folder} closed", "folder", d)
END SUB

SUB VerFilterChanged
  VerFillChanges
  VerLayOut
END SUB

' The pane's words and which of its parts show.
SUB VerShowState
  DIM vis AS INTEGER, k AS INTEGER, u AS STRING
  vis = Flag(Repo.Versioned <> 0)
  VerBranch.Visible = vis
  VerCount.Visible = vis
  VerSaveBtn.Visible = vis
  VerChangesHead.Visible = vis
  VerNoChanges.Visible = Flag(vis AND VerChCount = 0)
  VerChanges.Visible = Flag(vis AND VerChCount > 0)
  VerFilter.Visible = Flag(vis AND VerChCount > VERFILTERAT)
  VerTimeHead.Visible = vis
  VerTimeline.Visible = vis
  VerWhatBtn.Visible = vis
  VerBackBtn.Visible = vis
  VerUndoBtn.Visible = vis
  VerEmptyIcon.Visible = Flag(vis = 0)
  VerEmptyTitle.Visible = Flag(vis = 0)
  VerEmptyText.Visible = Flag(vis = 0)
  VerStartBtn.Visible = Flag(vis = 0 AND Repo.Available <> 0 AND ProjectOpen <> 0)
  VerLocalNote.Visible = Flag(vis = 0 AND ProjectOpen <> 0)
  VerLocalBtn.Visible = Flag(vis = 0 AND ProjectOpen <> 0)
  IF ProjectOpen THEN VerSubFull = ProjectName ELSE VerSubFull = TR$("No project open")
  IF ProjectOpen = 0 THEN
    VerEmptyTitle.Caption = TR$("No project open")
    VerEmptyText.Caption = TR$("Open a project to keep versions of it: every step of your work, so nothing is ever lost.")
    VerLocalNote.Caption = ""
  ELSEIF Repo.Available = 0 THEN
    VerEmptyTitle.Caption = TR$("Not in a browser")
    VerEmptyText.Caption = TR$("Versions need RapidR Studio on your computer: a browser can't keep them yet.")
    VerLocalNote.Caption = TR$("Studio still keeps a copy of each file every time you save it, in this browser. Local History shows them.")
  ELSEIF vis = 0 THEN
    VerEmptyTitle.Caption = TR$("No versions yet")
    VerEmptyText.Caption = TR$("A version keeps all your files as they are now, with a few words about what changed, so you can always go back. Git calls versions commits.")
    VerLocalNote.Caption = TR$("Until then Studio keeps a copy of each file every time you save it. Local History shows them.")
  ELSE
    ' (the chips: the experiment, and the changes or "All saved")
    VerBranchText.Caption = Repo.Experiment
    VerBranch.Hint = TR$("{experiment} is the experiment you're on (Git calls it a branch)", "experiment", Repo.Experiment)
    VerBranch.AccessibleName = TR$("Experiment: {experiment}", "experiment", Repo.Experiment)
    VerBranch.AccessibleDescription = VerBranch.Hint
    IF VerChCount = 0 THEN
      VerCountChip.Tag = 1
      VerCountText.Caption = TR$("All saved")
      VerCountText.Font.Color = TC("status.success.text")
      VerBmp(2).LoadIcon "success", 16
    ELSE
      VerCountChip.Tag = 0
      VerCountText.Caption = TRN$("{n} change", "{n} changes", VerChCount)
      VerCountText.Font.Color = TC("status.info.text")
      VerBmp(2).LoadIcon "info", 16
    END IF
    VerCountIcon.BMPHandle = VerBmp(2)
    VerCount.AccessibleName = VerChangesWords
    VerCountChip.Repaint
  END IF
  VerNoChanges.Caption = TR$("Everything is saved in a version.")
  u = Repo.CanUndo
  VerUndoBtn.Enabled = Flag(u <> "")
  IF u = "" THEN
    VerUndoNote.Caption = ""
    VerUndoNote.Hint = ""
    VerUndoBtn.Hint = TR$("Nothing to undo")
  ELSE
    VerUndoNote.Hint = TR$("Undo: {what}", "what", u)
    VerUndoBtn.Hint = VerUndoNote.Hint
  END IF
  VerUndoWanted = Flag(vis AND u <> "")
  VerUndoNote.Visible = VerUndoWanted
  VerUndoNote.AccessibleName = VerUndoNote.Hint
  VerUndoBtn.AccessibleDescription = VerUndoBtn.Hint
  k = VerTimeline.ItemIndex
  VerWhatBtn.Enabled = Flag(k >= 0 AND k < VerTimeCount)
  VerBackBtn.Enabled = Flag(k >= 0 AND k < VerTimeCount)
  VerSaveBtn.Enabled = Flag(VerChCount > 0 AND Repo.ReadOnly = 0)
  VerLayOut
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

' (a chip's parts for its words: 8, the icon, 6, the words, 12 — Width
' its whole width)
SUB VerChipLayOut(Chip AS RPanel, Ground AS RCanvas, Icon AS RImage, Words AS RLabel, X AS INTEGER, Y AS INTEGER)
  DIM ch AS INTEGER, cw AS INTEGER
  ch = UiCompactH
  cw = 8 + 16 + 6 + VerTextW(Words.Caption, 0) + 12
  Chip.Left = X: Chip.Top = Y: Chip.Width = cw: Chip.Height = ch
  Ground.Left = 0: Ground.Top = 0: Ground.Width = cw: Ground.Height = ch
  Icon.Left = 8: Icon.Top = (ch - 16) \ 2
  Words.Left = 30: Words.Top = (ch - Words.Height) \ 2: Words.Width = cw - 30 - 12
END SUB

' The pane's controls for its size (the design guide's pane anatomy):
'   - the header: the title and the project's name; the chips at the
'     right on its centre line, or on a row of their own under them when
'     the pane is narrow (one under the other when narrower still);
'   - without versions: an icon, a title, a line, Start (the one primary
'     button) and the local history's note and button;
'   - with them: "Changes" with Save a Version... at its right (or under
'     it), the filter (more than 20 changes), the list (up to about 40% of
'     the room); "Timeline", the timeline (the rest); What Changed?, Go
'     Back..., Undo — each as wide as its words, on as many rows as the
'     width needs —; what Undo takes back, in up to two lines.
' Every size comes from the width and the words only — nothing read back
' from what the layout set —, so a pane resized step by step moves its
' controls one way, never back and forth.
SUB VerLayOut
  DIM w AS INTEGER, h AS INTEGER, ch AS INTEGER, hh AS INTEGER, y AS INTEGER, x AS INTEGER, lh AS INTEGER, dh AS INTEGER
  DIM th AS INTEGER, top AS INTEGER, cw1 AS INTEGER, cw2 AS INTEGER, tw AS INTEGER, bw AS INTEGER, n AS INTEGER, rows AS INTEGER
  DIM rh AS INTEGER, need AS INTEGER, room AS INTEGER, i AS INTEGER, k AS INTEGER, ty AS INTEGER, s AS STRING, three AS INTEGER, rowh AS INTEGER
  DIM bwd(2) AS INTEGER, blockh AS INTEGER, noteh AS INTEGER, lines AS INTEGER
  ch = UiCompactH
  hh = UiPaneHeadH
  w = VerPanel.ClientWidth
  h = VerPanel.ClientHeight
  IF w < 120 THEN w = 120
  IF h < 160 THEN h = 160
  lh = VerLineH(0)
  dh = VerLineH(1)
  ' ---- the header
  th = lh + 2 + dh
  tw = VerTextW(VerHead.Caption, 1)
  IF VerTextW(VerSubFull, 2) > tw THEN tw = VerTextW(VerSubFull, 2)
  ty = (hh - th) \ 2
  top = hh
  x = w - 12
  IF VerBranch.Visible THEN
    cw1 = 8 + 16 + 6 + VerTextW(VerBranchText.Caption, 0) + 12
    cw2 = 8 + 16 + 6 + VerTextW(VerCountText.Caption, 0) + 12
    IF 12 + tw + 12 + cw1 + 8 + cw2 + 12 <= w THEN
      ' (beside the title, on the header's centre line)
      VerChipLayOut VerCount, VerCountChip, VerCountIcon, VerCountText, w - 12 - cw2, (hh - ch) \ 2
      VerChipLayOut VerBranch, VerBranchChip, VerBranchIcon, VerBranchText, w - 12 - cw2 - 8 - cw1, (hh - ch) \ 2
      x = VerBranch.Left - 12
    ELSEIF 12 + cw1 + 8 + cw2 + 12 <= w THEN
      ' (a row of their own under the title)
      ty = 10
      VerChipLayOut VerBranch, VerBranchChip, VerBranchIcon, VerBranchText, 12, ty + th + 8
      VerChipLayOut VerCount, VerCountChip, VerCountIcon, VerCountText, 12 + cw1 + 8, ty + th + 8
      top = ty + th + 8 + ch + 10
    ELSE
      ' (narrower still: one under the other)
      ty = 10
      VerChipLayOut VerBranch, VerBranchChip, VerBranchIcon, VerBranchText, 12, ty + th + 8
      VerChipLayOut VerCount, VerCountChip, VerCountIcon, VerCountText, 12, ty + th + 8 + ch + 6
      top = ty + th + 8 + 2 * ch + 6 + 10
    END IF
  END IF
  IF VerTop.Height <> top THEN VerTop.Height = top
  VerHead.Left = 12: VerHead.Top = ty: VerHead.Width = x - 12: VerHead.Height = lh
  VerSub.Left = 12: VerSub.Top = ty + lh + 2: VerSub.Width = x - 12: VerSub.Height = dh
  ' (a long project name cut with "…" to the room it has, whole in its tooltip)
  VerSub.Caption = VerWrap(VerSubFull, x - 12, 1, 2)
  IF VerSub.Caption <> VerSubFull THEN VerSub.Hint = VerSubFull ELSE VerSub.Hint = ""
  VerSub.ShowHint = Flag(VerSub.Hint <> "")
  y = top + 12
  ' ---- no versions: what they are, Start
  IF Repo.Versioned = 0 THEN
    VerEmptyIcon.Left = 12: VerEmptyIcon.Top = y
    y = y + 24 + 8
    lines = VerLinesOf(VerEmptyTitle.Caption, w - 24, 0, 1)
    VerEmptyTitle.Left = 12: VerEmptyTitle.Top = y: VerEmptyTitle.Width = w - 24: VerEmptyTitle.Height = lines * lh + 2
    y = y + lines * lh + 2 + 4
    lines = VerLinesOf(VerEmptyText.Caption, w - 24, 0, 0)
    VerEmptyText.Left = 12: VerEmptyText.Top = y: VerEmptyText.Width = w - 24: VerEmptyText.Height = lines * lh + 2
    y = y + lines * lh + 2 + 16
    IF VerStartBtn.Visible THEN
      ' (its whole words when they fit, else the short ones: its name for a
      ' screen reader stays whole)
      s = TR$("&Start Keeping Versions of This Project")
      IF VerBtnW(s) > w - 24 THEN s = TR$("&Start Keeping Versions")
      VerStartBtn.Caption = s
      bw = VerBtnW(s)
      IF bw > w - 24 THEN bw = w - 24
      VerStartBtn.Left = 12: VerStartBtn.Top = y: VerStartBtn.Width = bw: VerStartBtn.Height = ch
      y = y + ch + 24
    END IF
    IF VerLocalNote.Visible THEN
      lines = VerLinesOf(VerLocalNote.Caption, w - 24, 0, 2)
      VerLocalNote.Left = 12: VerLocalNote.Top = y: VerLocalNote.Width = w - 24: VerLocalNote.Height = lines * dh + 4
      y = y + lines * dh + 4 + 8
      bw = VerBtnW(VerLocalBtn.Caption)
      IF bw > w - 24 THEN bw = w - 24
      VerLocalBtn.Left = 12: VerLocalBtn.Top = y: VerLocalBtn.Width = bw: VerLocalBtn.Height = ch
    END IF
    EXIT SUB
  END IF
  ' ---- what goes under the changes, measured first: the buttons' rows,
  ' Undo's words (dropped when the pane is too short for them: Undo's
  ' tooltip and its description for a screen reader still say them), a
  ' version's row
  bwd(0) = VerBtnW(VerWhatBtn.Caption): bwd(1) = VerBtnW(VerBackBtn.Caption): bwd(2) = VerBtnW(VerUndoBtn.Caption)
  n = 1: x = 12
  FOR i = 0 TO 2
    IF bwd(i) > w - 24 THEN bwd(i) = w - 24
    IF x > 12 AND x + bwd(i) > w - 12 THEN
      n = n + 1
      x = 12
    END IF
    x = x + bwd(i) + 8
  NEXT
  blockh = n * ch + (n - 1) * 8
  noteh = 0
  IF VerUndoWanted THEN
    s = VerWrap(VerUndoNote.Hint, w - 24, 2, 2)
    VerUndoNote.Caption = s
    noteh = (TALLY(s, CHR$(10)) + 1) * dh + 4
  END IF
  ' (a version's row: its words on one line, or two when one of them needs
  ' it at this width; who and when under them — a scroll bar's room taken
  ' off when they may need it)
  room = (w - 16) - 4 - 36 - 8
  rowh = 8 + 2 * lh + 2 + 8
  three = 0
  FOR i = 0 TO VerTimeCount - 1
    IF VerTextW(VerTimeMsg(i), 0) > room - Flag(VerTimeCount > 1) * 16 THEN
      three = 1
      EXIT FOR
    END IF
  NEXT
  IF three THEN rowh = rowh + lh
  IF VerTimeline.ItemHeight <> rowh THEN VerTimeline.ItemHeight = rowh
  ' ---- the changes: "Changes" and Save a Version... beside it (or under it)
  bw = VerBtnW(VerSaveBtn.Caption)
  IF 12 + VerTextW(VerChangesHead.Caption, 1) + 12 + bw + 12 <= w THEN
    VerChangesHead.Left = 12: VerChangesHead.Top = y + (ch - lh) \ 2: VerChangesHead.Width = w - 36 - bw: VerChangesHead.Height = lh
    VerSaveBtn.Left = w - 12 - bw: VerSaveBtn.Top = y: VerSaveBtn.Width = bw: VerSaveBtn.Height = ch
    y = y + ch + 8
  ELSE
    VerChangesHead.Left = 12: VerChangesHead.Top = y: VerChangesHead.Width = w - 24: VerChangesHead.Height = lh
    y = y + lh + 6
    IF bw > w - 24 THEN bw = w - 24
    VerSaveBtn.Left = 12: VerSaveBtn.Top = y: VerSaveBtn.Width = bw: VerSaveBtn.Height = ch
    y = y + ch + 8
  END IF
  ' (what the timeline's part needs at least: its title, one version, the
  ' buttons; Undo's words when there is room)
  need = lh + 6 + rowh + 4 + 8 + blockh + 12
  IF VerChCount = 0 THEN
    lines = VerLinesOf(VerNoChanges.Caption, w - 24, 0, 0)
    VerNoChanges.Left = 12: VerNoChanges.Top = y: VerNoChanges.Width = w - 24: VerNoChanges.Height = lines * lh + 2
    y = y + lines * lh + 2 + 16
  ELSE
    IF VerFilter.Visible THEN
      VerFilter.Left = 12: VerFilter.Top = y: VerFilter.Width = w - 24: VerFilter.Height = ch
      y = y + ch + 8
    END IF
    ' (up to about 40% of the room left, three rows at least — fewer when
    ' there are fewer, or when the pane is short)
    rh = VerChanges.ItemHeight
    rows = ((h - y) * 4 \ 10) \ rh
    IF rows < 3 THEN rows = 3
    IF rows > VerRowCount THEN rows = VerRowCount
    k = (h - y - 16 - need - Flag(noteh > 0) * (6 + noteh) - 4) \ rh
    IF rows > k THEN rows = k
    IF rows < 1 THEN rows = 1
    VerChanges.Left = 8: VerChanges.Top = y: VerChanges.Width = w - 16: VerChanges.Height = rows * rh + 4
    y = y + rows * rh + 4 + 16
  END IF
  ' (Undo's words: only when they fit with one version's row above them)
  VerUndoNote.Visible = Flag(noteh > 0 AND h - y - need - 6 - noteh >= 0)
  IF VerUndoNote.Visible THEN blockh = blockh + 6 + noteh
  ' ---- the timeline
  VerTimeHead.Left = 12: VerTimeHead.Top = y: VerTimeHead.Width = w - 24: VerTimeHead.Height = lh
  y = y + lh + 6
  th = h - y - 8 - blockh - 12
  IF th < rowh + 4 THEN th = rowh + 4
  VerTimeline.Left = 8: VerTimeline.Top = y: VerTimeline.Width = w - 16: VerTimeline.Height = th
  y = y + th + 8
  ' (What Changed?, Go Back..., Undo: each as wide as its words, 8 apart,
  ' a new row when the next doesn't fit)
  x = 12
  FOR i = 0 TO 2
    IF x > 12 AND x + bwd(i) > w - 12 THEN
      x = 12
      y = y + ch + 8
    END IF
    SELECT CASE i
      CASE 0: VerWhatBtn.Left = x: VerWhatBtn.Top = y: VerWhatBtn.Width = bwd(i): VerWhatBtn.Height = ch
      CASE 1: VerBackBtn.Left = x: VerBackBtn.Top = y: VerBackBtn.Width = bwd(i): VerBackBtn.Height = ch
      CASE 2: VerUndoBtn.Left = x: VerUndoBtn.Top = y: VerUndoBtn.Width = bwd(i): VerUndoBtn.Height = ch
    END SELECT
    x = x + bwd(i) + 8
  NEXT
  y = y + ch + 6
  IF VerUndoNote.Visible THEN
    VerUndoNote.Left = 12: VerUndoNote.Top = y: VerUndoNote.Width = w - 24: VerUndoNote.Height = noteh
  END IF
END SUB

' (Text as it fits Room pixels of the lists' font: whole, or cut with "…")
FUNCTION VerFit(Text AS STRING, Room AS INTEGER) AS STRING
  DIM k AS INTEGER
  VerMeasure.Font.Name = UIFont.Name: VerMeasure.Font.Size = UIFont.Size: VerMeasure.Font.Bold = 0
  IF VerMeasure.TextWidth(Text) <= Room OR LEN(Text) < 4 THEN
    Result = Text
    EXIT FUNCTION
  END IF
  FOR k = LEN(Text) - 1 TO 1 STEP -1
    IF VerMeasure.TextWidth(LEFT$(Text, k) + "…") <= Room THEN EXIT FOR
  NEXT
  Result = LEFT$(Text, k) + "…"
END FUNCTION

' (…cut in the middle: a file's name keeps its start and its extension)
FUNCTION VerFitMiddle(Text AS STRING, Room AS INTEGER) AS STRING
  DIM k AS INTEGER, s AS STRING
  VerMeasure.Font.Name = UIFont.Name: VerMeasure.Font.Size = UIFont.Size: VerMeasure.Font.Bold = 0
  IF VerMeasure.TextWidth(Text) <= Room OR LEN(Text) < 6 THEN
    Result = Text
    EXIT FUNCTION
  END IF
  s = "…"
  FOR k = LEN(Text) - 1 TO 2 STEP -1
    s = LEFT$(Text, k - k \ 2) + "…" + RIGHT$(Text, k \ 2)
    IF VerMeasure.TextWidth(s) <= Room THEN EXIT FOR
  NEXT
  Result = s
END FUNCTION

' (a short "when": "just now", "3 min ago", "2 h ago", "yesterday", "5 days ago")
FUNCTION VerAgoShort(Seconds AS LONG) AS STRING
  DIM d AS LONG
  d = Repo.Now - Seconds
  IF d < 60 THEN
    Result = TR$("just now")
  ELSEIF d < 3600 THEN
    Result = TR$("{n} min ago", "n", Num(d \ 60))
  ELSEIF d < 86400 THEN
    Result = TR$("{n} h ago", "n", Num(d \ 3600))
  ELSEIF d < 172800 THEN
    Result = TR$("yesterday")
  ELSE
    Result = TRN$("{n} day ago", "{n} days ago", d \ 86400)
  END IF
END FUNCTION

' A version's row on the timeline: a line through a dot for each version
' (the newest — the version your files are based on — filled in the
' accent; an AI's checkpoint a small, lighter one), the words in up to two
' lines, then who and when, dimmed; the row picked in the selection's
' colours, the whole width.
SUB VerDrawTime(Index AS INTEGER, State AS INTEGER, R AS RRect)
  DIM bg AS LONG, fg AS LONG, dm AS LONG, lc AS LONG, ac AS LONG, cx AS INTEGER, cy AS INTEGER, rr AS INTEGER, sel AS INTEGER, room AS INTEGER
  DIM lh AS INTEGER, x AS INTEGER, y AS INTEGER, t AS STRING, n AS INTEGER, i AS INTEGER, meta AS STRING, cur AS STRING
  IF Index < 0 OR Index >= VerTimeCount THEN EXIT SUB
  sel = Flag(Index = VerTimeline.ItemIndex)
  IF sel THEN
    bg = TC("selected"): fg = TC("selected.text"): dm = TC("selected.text"): ac = TC("selected.text")
  ELSE
    bg = TC("window"): fg = TC("text"): dm = TC("page.dim"): ac = TC("page.link")
  END IF
  lc = TC("border")
  VerTimeline.FillRect R.Left, R.Top, R.Right, R.Bottom, bg
  lh = VerLineH(0)
  cx = R.Left + 18: cy = R.Top + 8 + lh \ 2
  ' (the line through the dots: from the row above, to the row below)
  IF Index > 0 THEN VerTimeline.Line cx, R.Top, cx, cy, lc
  IF Index < VerTimeCount - 1 THEN VerTimeline.Line cx, cy, cx, R.Bottom, lc
  x = R.Left + 36
  room = R.Right - x - 8
  IF VerTimeKind(Index) = "checkpoint" THEN
    rr = 3
    VerTimeline.Circle cx - rr, cy - rr, cx + rr + 1, cy + rr + 1, dm, dm
    t = VerWrap(TR$("AI checkpoint: {what}", "what", VerTimeMsg(Index)), room, 2, 0)
  ELSE
    rr = 5
    IF Index = 0 THEN
      ' (the current version: a ring and a dot in the accent)
      VerTimeline.Circle cx - rr - 1, cy - rr - 1, cx + rr + 2, cy + rr + 2, ac, bg
      VerTimeline.Circle cx - rr, cy - rr, cx + rr + 1, cy + rr + 1, ac, bg
      VerTimeline.Circle cx - 2, cy - 2, cx + 3, cy + 3, ac, ac
    ELSE
      VerTimeline.Circle cx - rr, cy - rr, cx + rr + 1, cy + rr + 1, dm, bg
    END IF
    t = VerWrap(VerTimeMsg(Index), room, 2, 0)
  END IF
  n = TALLY(t, CHR$(10)) + 1
  y = R.Top + 8
  FOR i = 1 TO n
    IF VerTimeKind(Index) = "checkpoint" THEN
      VerTimeline.TextOut x, y, FIELD$(t, CHR$(10), i), dm, -1
    ELSE
      VerTimeline.TextOut x, y, FIELD$(t, CHR$(10), i), fg, -1
    END IF
    y = y + lh
  NEXT
  y = y + 2
  ' (who and when; the newest says it's the current version)
  IF VerTimeKind(Index) = "checkpoint" THEN
    meta = VerAgoShort(VerTimeWhen(Index))
  ELSE
    meta = TR$("{who} · {when}", "who", VerTimeWho(Index), "when", VerAgoShort(VerTimeWhen(Index)))
  END IF
  ' (the newest: "Current", when it fits with who and when — the ring says
  ' it too, and a screen reader hears "the current version")
  cur = TR$("Current") + " · "
  IF Index = 0 AND VerTimeKind(Index) <> "checkpoint" AND VerTextW(cur + meta, 0) <= room THEN
    VerTimeline.TextOut x, y, TR$("Current"), ac, -1
    x = x + VerTextW(TR$("Current"), 0)
    VerTimeline.TextOut x, y, " · " + meta, dm, -1
  ELSE
    VerTimeline.TextOut x, y, VerFit(meta, room), dm, -1
  END IF
END SUB

' A row of the changes: a folder's — its arrow (open or closed), its name,
' how many files changed in it —, or a file's: its name (cut in the
' middle when it must be), then its change in words on a soft chip at the
' right — New, Changed, Deleted —; the row picked in the selection's
' colours, the whole width.
SUB VerDrawChange(Index AS INTEGER, State AS INTEGER, R AS RRect)
  DIM bg AS LONG, fg AS LONG, dm AS LONG, k AS STRING, chipword AS STRING, sel AS INTEGER, lh AS INTEGER, ty AS INTEGER
  DIM x AS INTEGER, cx AS INTEGER, cy AS INTEGER, c AS INTEGER, cw AS INTEGER, x1 AS INTEGER, x2 AS INTEGER, y1 AS INTEGER, i AS INTEGER, d AS INTEGER, num AS STRING
  IF Index < 0 OR Index >= VerRowCount THEN EXIT SUB
  sel = Flag(Index = VerChanges.ItemIndex)
  IF sel THEN
    bg = TC("selected"): fg = TC("selected.text"): dm = TC("selected.text")
  ELSE
    bg = TC("window"): fg = TC("text"): dm = TC("page.dim")
  END IF
  VerChanges.FillRect R.Left, R.Top, R.Right, R.Bottom, bg
  lh = VerLineH(0)
  ty = R.Top + (R.Bottom - R.Top - lh) \ 2
  IF VerRowFolder(Index) THEN
    d = VerRowRef(Index)
    ' (the arrow: › closed, ⌄ open — two strokes)
    cx = R.Left + 14: cy = R.Top + (R.Bottom - R.Top) \ 2
    FOR c = 0 TO 1
      IF VerDirShut(d) THEN
        VerChanges.Line cx - 2 + c, cy - 4, cx + 2 + c, cy, fg
        VerChanges.Line cx + 2 + c, cy, cx - 2 + c, cy + 4, fg
      ELSE
        VerChanges.Line cx - 4, cy - 2 + c, cx, cy + 2 + c, fg
        VerChanges.Line cx, cy + 2 + c, cx + 4, cy - 2 + c, fg
      END IF
    NEXT
    num = Num(VerDirFiles(d))
    x2 = R.Right - 10 - VerTextW(num, 0)
    VerChanges.TextOut x2, ty, num, dm, -1
    VerChanges.TextOut R.Left + 28, ty, VerFitMiddle(VerDir(d) + "/", x2 - 8 - (R.Left + 28)), fg, -1
    EXIT SUB
  END IF
  i = VerRowRef(Index)
  SELECT CASE VerChLetter(i)
    CASE "N": k = "status.success": chipword = TR$("New")
    CASE "D": k = "status.danger": chipword = TR$("Deleted")
    CASE ELSE: k = "status.info": chipword = TR$("Changed")
  END SELECT
  ' (the chip: a soft pill at the right, its chipword in the status's colour)
  cw = VerTextW(chipword, 0) + 16
  x2 = R.Right - 8: x1 = x2 - cw
  y1 = R.Top + (R.Bottom - R.Top - 18) \ 2
  VerChanges.RoundRect x1, y1, x2, y1 + 18, 18, 18, TC(k + ".border")
  VerChanges.RoundRect x1 + 1, y1 + 1, x2 - 1, y1 + 17, 16, 16, TC(k)
  VerChanges.TextOut x1 + 8, y1 + (18 - lh) \ 2, chipword, TC(k + ".text"), -1
  IF VerDir(VerChDir(i)) = "" THEN x = R.Left + 12 ELSE x = R.Left + 28
  VerChanges.TextOut x, ty, VerFitMiddle(FileTitle(VerChPath(i)), x1 - 8 - x), fg, -1
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

' (a double click: a folder opened or closed, a changed file opened)
SUB VerChangeOpen
  DIM k AS INTEGER, i AS INTEGER
  k = VerChanges.ItemIndex
  IF k < 0 OR k >= VerRowCount THEN EXIT SUB
  IF VerRowFolder(k) THEN
    VerToggle k, -1
    EXIT SUB
  END IF
  i = VerRowRef(k)
  IF VerChLetter(i) <> "D" THEN OpenDocument Proj.Folder + "/" + VerChPath(i)
END SUB

' (the keys: Right opens a folder, Left closes it — on a file, goes to its
' folder —, Enter and Space open or close a folder, Enter opens a file)
SUB VerChangeKey(Key AS INTEGER, Shift AS INTEGER)
  DIM k AS INTEGER, j AS INTEGER
  k = VerChanges.ItemIndex
  IF k < 0 OR k >= VerRowCount THEN EXIT SUB
  SELECT CASE Key
    CASE 39: IF VerRowFolder(k) THEN VerToggle k, 1
    CASE 37
      IF VerRowFolder(k) THEN
        VerToggle k, 0
      ELSE
        FOR j = k - 1 TO 0 STEP -1
          IF VerRowFolder(j) THEN
            VerChanges.ItemIndex = j
            VerChanges.Repaint
            EXIT FOR
          END IF
        NEXT
      END IF
    CASE 32: IF VerRowFolder(k) THEN VerToggle k, -1
    CASE 13: VerChangeOpen
  END SELECT
END SUB

' (a click on a folder's arrow opens or closes it)
SUB VerChangeMouse(Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  DIM k AS INTEGER
  k = VerChanges.ItemIndex
  IF Button <> 0 OR X > 30 THEN EXIT SUB
  IF k >= 0 AND k < VerRowCount THEN
    IF VerRowFolder(k) THEN VerToggle k, -1
  END IF
END SUB

' ---- the actions ------------------------------------------------------------------------------

' View > Versions: the pane, to the front, the keyboard in it.
SUB VerShowPane
  IF Dock.PaneState("verpanel") = "hidden" THEN Dock.ShowPane("verpanel")
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
  IF Dock.PaneState("verpanel") <> "hidden" THEN Dock.ShowPane("verpanel")
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
      CASE "D": VerFilesList.AddSubItem i, TR$("deleted")
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
  Reg.RootKey = HKCU
  Reg.OpenKey(STUDIO_KEY, 1)
  Reg.WriteString("VersionsName", VerAuthor)
  Reg.CloseKey
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

' (the list's copies as wide as their words need, when the pane has room: a
' longer language, "Como estaba en el disco, hace 3 minutos": nothing cut off)
SUB VerDiffLayOut
  DIM w AS INTEGER, h AS INTEGER, ch AS INTEGER, hh AS INTEGER, bw AS INTEGER, lw AS INTEGER, x AS INTEGER, th AS INTEGER, i AS INTEGER, tw AS INTEGER, most AS INTEGER
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
  IF w >= 600 THEN
    VerMeasure.Font.Name = UIFont.Name: VerMeasure.Font.Size = UIFont.Size
    FOR i = 0 TO VerDiffList.ItemCount - 1
      tw = VerMeasure.TextWidth(VerDiffList.Item(i)) + 30
      IF tw > most THEN most = tw
    NEXT
    IF most > lw THEN lw = most
    IF lw > w \ 2 THEN lw = w \ 2
  END IF
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

' (project.inc: a project opened or closed — the Versions tab, always
' beside Project, says it; the folders' open and closed rows start again)
SUB VersionsProjectChanged
  VerBadged = ""
  VerShut = ""
  VerOpened = ""
  VerFilter.Text = ""
  ProjectTree.ClearBadges
  VerRefresh
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
    CASE "saveonly"
      ' (versions:saveonly=<message>;<file>;<file>…: those files only, as
      ' Save Version with the others unticked)
      IF Repo.Versioned = 0 THEN VerStart
      IF Repo.SaveVersion(FIELD$(v, ";", 1), REPLACESUBSTR$(MID$(v, INSTR(v, ";") + 1), ";", CHR$(10)) + CHR$(10), 0) = "" THEN OutputLine "versions:saveonly: " + Repo.Error
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
    CASE "width"
      ' (versions:width=<pixels>: the side column the Versions tab is in made
      ' that wide, as its splitter dragged would)
      Dock.PaneExtent("verpanel", "width", VAL(v))
    CASE "oldlayout"
      ' (versions:oldlayout: the layout as a Studio from before Versions was
      ' a tab of its own saved it — no Versions pane in it —, read again as
      ' at start)
      v = REPLACESUBSTR$(Dock.SaveLayout, " verpanel", "")
      Dock.LoadLayout(v)
      VerLayoutLoaded
    CASE "problems"
      ' (versions:problems: what the layout report finds wrong with the
      ' Versions pane's controls — text cut off, a control past the pane's
      ' edge or on top of another —, a line each in the Output)
      v = VerEyes.Problems("")
      k = 0
      FOR i = 1 TO TALLY(v, CHR$(10)) + 1
        what = FIELD$(v, CHR$(10), i)
        IF LEFT$(LCASE$(what), 3) = "ver" OR INSTR(LCASE$(what), " and ver") > 0 THEN
          OutputLine "[versions] problem: " + what
          k = k + 1
        END IF
      NEXT
      OutputLine "[versions] problems=" + Num(k)
    CASE "report"
      ' (versions:report=<file>: what the pane, the Project pane's badge of
      ' that file and the status bar say)
      OutputLine "[versions] versioned=" + Num(Flag(Repo.Versioned <> 0)) + " changes=" + Num(VerChCount) + " timeline=" + Num(VerTimeCount) + " experiment=" + Repo.Experiment + " badge=" + ProjectTree.FileBadge(v) + " status=" + Status.Panel(1).Caption
  END SELECT
END SUB

' external.inc: files changed outside Studio (lane EXTERNAL-CHANGES;
' Preview 5: "Never lose anything"). Another AI agent, an editor, git
' checking out a branch: a file Studio has open changes on disk.
'
'   - Studio looks (RPROJECT's ExternalChanges: the time, size and a hash of
'     each open file and of the project file) when its window is in front
'     again, and every two seconds while it is;
'   - a file with no unsaved changes here is loaded again quietly: what the
'     editor had is kept in the local history first ("before reload"), and
'     stays in the editor's Undo (one Ctrl+Z brings it back); the status
'     bar says "main.rr was updated outside Studio" (a screen reader too);
'   - a file with unsaved changes here gets a question strip above its
'     editor and designer (the design guide's: a card, its 4-pixel mark in
'     the warning colour): Compare (the disk's version beside yours), Use
'     Disk Version (yours kept in the local history first; Undo still
'     brings it back), Keep Mine;
'   - Save never overwrites newer content on disk silently: it asks first
'     ("main.rr changed outside Studio since you opened it. Save anyway?",
'     Cancel the default);
'   - a file deleted outside keeps its tab, marked "(deleted on disk)", and
'     its text; Save makes it again (asked first);
'   - the project file changed (an agent added or removed files): the
'     Project pane follows quietly;
'   - many files at once (git checkout, an agent rewriting 20 files): one
'     note, "12 files were updated outside Studio", with Show (the list, in
'     the Output);
'   - Studio's own writes (Save, RAI's Apply, an AI app's changes, Build,
'     Versions' Go Back) never count: Studio watches what it wrote.
' Studio on the web: files live in the page's store and change only
' through Studio — the checks find nothing there.
' File > Reopen Project closes the project and opens it fresh.

DECLARE SUB ExtTick
DECLARE SUB ExtLayOut
DECLARE SUB ExtCompareClick
DECLARE SUB ExtDiskClick
DECLARE SUB ExtMineClick
DECLARE SUB ExtShowClick
DECLARE SUB ExtCloseClick
DECLARE SUB ExtShowBar
DECLARE SUB ExtReload(d AS INTEGER, Asked AS INTEGER)
DECLARE SUB ExtProjectFile(What AS STRING)
DECLARE FUNCTION ExtButtonW(B AS RButton) AS INTEGER

' (Studio was in the background at the last tick: a look once it is back)
DIM ExtAway AS INTEGER
DIM ExtTicks AS INTEGER
' (a look, a question, a reload going on: the timer waits)
DIM ExtBusy AS INTEGER
' (DocExtAsk, DocGone: decl.inc)
' (the strip: on which document, saying what — "ask" a question, "note"
' many files updated)
DIM ExtBarDoc AS INTEGER
DIM ExtBarMode AS STRING
' (the last batch of files updated quietly: their paths, a line each; the
' note waiting to be shown on the next document shown)
DIM ExtNoteFiles AS STRING
DIM ExtNoteCount AS INTEGER
DIM ExtNoteOn AS INTEGER
DIM ExtBmp(2) AS RBitmap
' (tests: the folder an outside writer and Studio meet in — RAPIDR_TEST_EXT)
DIM ExtMarker AS RFileStream

CREATE ExtTimer AS RTimer
  Interval = 500
  Enabled = 0
  OnTimer = ExtTick
END CREATE

' The strip above a document (RDOCKMANAGER's DocumentBar).
CREATE ExtBar AS RPanel
  BevelOuter = 0
  Visible = 0
  Height = 70
  CREATE ExtBarMark AS RPanel
    Left = 0: Top = 0: Width = 4: Height = 70
    BevelOuter = 0
  END CREATE
  CREATE ExtBarIcon AS RImage
    Left = 16: Top = 11: Width = 16: Height = 16
    AccessibleHidden = 1
  END CREATE
  CREATE ExtBarText AS RLabel
    Left = 40: Top = 10: Height = 18
    AutoSize = 0
  END CREATE
  CREATE ExtCompare AS RButton
    Caption = TR$("&Compare")
    AccessibleDescription = TR$("Shows the disk's version beside yours")
    OnClick = ExtCompareClick
  END CREATE
  CREATE ExtDisk AS RButton
    Caption = TR$("&Use Disk Version")
    AccessibleDescription = TR$("Loads the file as it is on disk. Your version is kept in the local history, and Undo brings it back.")
    OnClick = ExtDiskClick
  END CREATE
  CREATE ExtMine AS RButton
    Caption = TR$("&Keep Mine")
    AccessibleDescription = TR$("Keeps your version here. Saving it will ask before it replaces the disk's version.")
    OnClick = ExtMineClick
  END CREATE
  CREATE ExtShow AS RButton
    Caption = TR$("&Show")
    AccessibleDescription = TR$("Lists the files in the Output")
    OnClick = ExtShowClick
  END CREATE
  CREATE ExtClose AS RButton
    Caption = TR$("C&lose")
    AccessibleName = TR$("Close the note")
    OnClick = ExtCloseClick
  END CREATE
END CREATE

SUB SetUpExternal
  ExtBarDoc = -1
  ExtBar.OnResize = ExtLayOut
  ExtTimer.Enabled = 1
END SUB

SUB ApplyExternalChrome
  DIM d AS INTEGER
  ExtBar.Color = TC("card")
  ExtBarText.Font = UIFont
  ExtBarText.Font.Bold = 1
  ExtBarText.Font.Color = TC("page.text")
  ExtBarText.Transparent = 1
  ExtCompare.Font = UIFont
  ExtDisk.Font = UIFont
  ExtMine.Font = UIFont
  ExtShow.Font = UIFont
  ExtClose.Font = UIFont
  ExtBmp(0).LoadIcon "warning", 16
  ExtBmp(1).LoadIcon "info", 16
  ' (the strip again: its words in the language now, its colours; a
  ' deleted file's tab says so in it too)
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) AND DocGone(d) THEN DocTitles d
  NEXT
  ExtShowBar
END SUB

' (a compact button as wide as its words: 12 each side, 72 at least)
FUNCTION ExtButtonW(B AS RButton) AS INTEGER
  Result = UITextWidth(B.Caption) + 24
  IF Result < 72 THEN Result = 72
END FUNCTION

' The strip: the icon and the words on one line, the buttons under them,
' compact, 8 apart, from where the words start.
SUB ExtLayOut
  DIM w AS INTEGER, ch AS INTEGER, x AS INTEGER, y AS INTEGER, h AS INTEGER
  ch = UiCompactH
  w = ExtBar.ClientWidth
  IF w < 200 THEN w = 600
  ExtBarIcon.Left = 16: ExtBarIcon.Top = 11
  ExtBarText.Left = 40: ExtBarText.Top = 10: ExtBarText.Width = w - 52: ExtBarText.Height = 18
  y = 10 + 18 + 8
  x = 16
  IF ExtCompare.Visible THEN
    ExtCompare.Left = x: ExtCompare.Top = y: ExtCompare.Width = ExtButtonW(ExtCompare): ExtCompare.Height = ch
    x = x + ExtCompare.Width + 8
  END IF
  IF ExtDisk.Visible THEN
    ExtDisk.Left = x: ExtDisk.Top = y: ExtDisk.Width = ExtButtonW(ExtDisk): ExtDisk.Height = ch
    x = x + ExtDisk.Width + 8
  END IF
  IF ExtMine.Visible THEN
    ExtMine.Left = x: ExtMine.Top = y: ExtMine.Width = ExtButtonW(ExtMine): ExtMine.Height = ch
    x = x + ExtMine.Width + 8
  END IF
  IF ExtShow.Visible THEN
    ExtShow.Left = x: ExtShow.Top = y: ExtShow.Width = ExtButtonW(ExtShow): ExtShow.Height = ch
    x = x + ExtShow.Width + 8
  END IF
  IF ExtClose.Visible THEN
    ExtClose.Left = x: ExtClose.Top = y: ExtClose.Width = ExtButtonW(ExtClose): ExtClose.Height = ch
  END IF
  h = y + ch + 10
  IF ExtBar.Height <> h THEN ExtBar.Height = h
  ExtBarMark.Left = 0: ExtBarMark.Top = 0: ExtBarMark.Height = h
END SUB

' The strip shows what the document in front needs: its question when its
' file changed on disk under unsaved changes, else the note of many files
' updated, else nothing.
SUB ExtShowBar
  DIM d AS INTEGER, mode AS STRING, t AS STRING
  d = ActiveCode
  IF d >= 0 THEN
    IF DocExtAsk(d) THEN
      mode = "ask"
    ELSEIF ExtNoteOn THEN
      mode = "note"
    END IF
  END IF
  ' (off the document it was on)
  IF ExtBarDoc >= 0 AND (ExtBarDoc <> d OR mode = "") THEN
    IF ExtBarDoc < DocCount THEN
      IF DocOpen(ExtBarDoc) THEN Dock.DocumentBar(DocPane(ExtBarDoc), "")
    END IF
    ExtBarDoc = -1
  END IF
  ExtBarMode = mode
  IF mode = "" THEN
    ExtBar.Visible = 0
    EXIT SUB
  END IF
  IF mode = "ask" THEN
    t = TR$("{file} changed outside Studio.", "file", FileTitle(DocFile(d)))
    ExtBarText.Caption = t
    ExtBar.AccessibleName = TR$("{file} changed outside Studio, and you have unsaved changes in it: Compare, Use Disk Version or Keep Mine.", "file", FileTitle(DocFile(d)))
    ExtBarMark.Color = TC("status.warning.border")
    ExtBarIcon.BMPHandle = ExtBmp(0)
  ELSE
    t = TRN$("{n} file was updated outside Studio.", "{n} files were updated outside Studio.", ExtNoteCount)
    ExtBarText.Caption = t
    ExtBar.AccessibleName = t
    ExtBarMark.Color = TC("status.info.border")
    ExtBarIcon.BMPHandle = ExtBmp(1)
  END IF
  ExtBarText.AccessibleHidden = 1
  ExtCompare.Visible = Flag(mode = "ask")
  ExtDisk.Visible = Flag(mode = "ask")
  ExtMine.Visible = Flag(mode = "ask")
  ExtShow.Visible = Flag(mode = "note")
  ExtClose.Visible = Flag(mode = "note")
  ExtLayOut
  ExtBar.Visible = 1
  ExtBarDoc = d
  Dock.DocumentBar(DocPane(d), "extbar")
END SUB

' ---- looking ----------------------------------------------------------------------------------

' Twice a second: nothing while Studio is in the background; a look when it
' is in front again, and every two seconds while it stays.
SUB ExtTick
  IF ExtBusy OR Loading THEN EXIT SUB
  IF Application.Active = 0 THEN
    ExtAway = 1
    EXIT SUB
  END IF
  IF ExtAway THEN
    ExtAway = 0
    ExtTicks = 0
    ExtCheck
    EXIT SUB
  END IF
  ExtTicks = ExtTicks + 1
  IF ExtTicks >= 4 THEN
    ExtTicks = 0
    ExtCheck
  END IF
END SUB

' What changed on disk since the last look, all of it together: files with
' no unsaved changes loaded again (one note for all of them), a question
' for each with unsaved changes, deleted files marked, the project file
' followed.
SUB ExtCheck
  DIM t AS STRING, n AS INTEGER, i AS INTEGER, row AS STRING, what AS STRING, path AS STRING, d AS INTEGER
  DIM done AS STRING, doneN AS INTEGER, asked AS INTEGER, gone AS INTEGER, said AS STRING
  IF ExtBusy THEN EXIT SUB
  t = Proj.ExternalChanges()
  IF t = "" THEN EXIT SUB
  ExtBusy = 1
  n = TALLY(t, CHR$(10))
  FOR i = 1 TO n
    row = FIELD$(t, CHR$(10), i)
    what = FIELD$(row, CHR$(9), 1)
    path = FIELD$(row, CHR$(9), 2)
    d = DocOf(path)
    IF ProjectOpen AND LCASE$(path) = LCASE$(ProjectFile) AND d < 0 THEN
      ExtProjectFile what
    ELSEIF d >= 0 THEN
      IF what = "deleted" THEN
        ' (deleted: the tab stays, marked, its text kept — Save makes it again)
        DocGone(d) = 1
        DocExtAsk(d) = 0
        DocDirty(d) = 1
        ' (its text is only here now: a copy in the local history, and in
        ' the copies of unsaved work Restore offers after a crash)
        IF ProjectOpen THEN
          IF Repo.Folder = "" THEN Repo.Folder = Proj.Folder
          IF Repo.KeepCopy(path, CodeDoc(d).Text, "deleted-outside") = "" AND Repo.Error <> "" THEN OutputLine TR$("Local history: {why}", "why", Repo.Error)
        END IF
        KeepNoted d
        DocTitles d
        gone = gone + 1
        said = TR$("{file} was deleted outside Studio: its text is kept here", "file", FileTitle(path))
        OutputLine said
      ELSE
        IF DocGone(d) THEN
          ' (there again: changed here only if typed in)
          DocGone(d) = 0
          DirtyFollows d
          DocTitles d
        END IF
        IF Proj.FileText(path) = CodeDoc(d).Text THEN
          ' (back as the editor has it: nothing to load)
          DocExtAsk(d) = 0
          Proj.Watch path
          DirtyFollows d
        ELSEIF DocDirty(d) THEN
          ' (unsaved changes here: asked, never overwritten)
          DocExtAsk(d) = 1
          asked = asked + 1
          said = TR$("{file} changed outside Studio, and you have unsaved changes in it", "file", FileTitle(path))
          OutputLine said
        ELSE
          ExtReload d, 0
          done = done + path + CHR$(10)
          doneN = doneN + 1
        END IF
      END IF
    END IF
  NEXT
  IF doneN = 1 THEN
    said = TR$("{file} was updated outside Studio", "file", FileTitle(FIELD$(done, CHR$(10), 1)))
    OutputLine said
    SetState RunState, said
  ELSEIF doneN > 1 THEN
    said = TRN$("{n} file was updated outside Studio", "{n} files were updated outside Studio", doneN)
    OutputLine said
    SetState RunState, said
    ExtNoteFiles = done
    ExtNoteCount = doneN
    ExtNoteOn = 1
  ELSEIF asked + gone > 0 THEN
    SetState RunState, said
  END IF
  IF said <> "" THEN Application.Announce said
  AnalyzeActive
  UpdateCommands
  ExtShowBar
  ExtBusy = 0
END SUB

' Document d's file loaded again from the disk: what the editor had kept in
' the local history first, then the disk's text put in as one undo step
' (Ctrl+Z brings the editor's back), the editor's saved point the disk's.
SUB ExtReload(d AS INTEGER, Asked AS INTEGER)
  DIM path AS STRING, disk AS STRING, patch AS STRING
  path = DocFile(d)
  disk = Proj.FileText(path)
  IF ProjectOpen THEN
    IF Repo.Folder = "" THEN Repo.Folder = Proj.Folder
    IF Repo.KeepCopy(path, CodeDoc(d).Text, "before-reload") = "" AND Repo.Error <> "" THEN OutputLine TR$("Local history: {why}", "why", Repo.Error)
  END IF
  patch = AITools.Patch(CodeDoc(d).Text, disk)
  IF patch <> "" THEN
    IF CodeDoc(d).ApplyPatches(patch, 0) = 0 THEN
      ' (the editor refused the patch: the file read whole — Undo can't go
      ' back past it, but the local history has the editor's text)
      Loading = 1
      CodeDoc(d).LoadFromFile path
      Loading = 0
    END IF
  END IF
  ' (what the disk has is what is saved now)
  CodeDoc(d).Modified = 0
  DocDirty(d) = 0
  DocExtAsk(d) = 0
  Proj.Watch path
  ' (no copy of unsaved work needed; the designer, the Project pane's
  ' components, a Markdown file's preview follow the new text)
  KeepForget d
  IF DocHasForm(d) THEN DesignRead d
  IF DocIsMd(d) THEN MdDoc(d).Text = CodeDoc(d).Text
  DocTitles d
  ProjectFileChanged d
  IF IsBasicFile(path) THEN Lang.Update path, CodeDoc(d).Text
END SUB

' The project file changed outside (an agent added or removed files): read
' again, the Project pane following quietly. Deleted: said; Save All makes
' it again.
SUB ExtProjectFile(What AS STRING)
  IF What = "deleted" THEN
    OutputLine TR$("{file} was deleted outside Studio: Save All makes it again", "file", FileTitle(ProjectFile))
    EXIT SUB
  END IF
  IF Proj.Open(ProjectFile) THEN
    Proj.Watch ProjectFile
    ProjectName = Proj.Name
    FillProjectTree
    UpdateTitle
    OutputLine TR$("{file} was updated outside Studio: the Project pane follows it", "file", FileTitle(ProjectFile))
  ELSE
    OutputLine TR$("Can't read the project again: {error}", "error", Proj.Error)
  END IF
END SUB

' ---- the question's answers ------------------------------------------------------------------

' Compare: the disk's version beside the editor's, in the What Changed
' document (versions.inc).
SUB ExtCompareClick
  IF ExtBarDoc < 0 THEN EXIT SUB
  ExtCompareDoc ExtBarDoc
END SUB

SUB ExtCompareDoc(d AS INTEGER)
  VerDiffFile = DocFile(d)
  VerShowDiff "disk"
END SUB

' Use Disk Version: the editor's text kept in the local history, then the
' disk's loaded (Undo still brings it back).
SUB ExtDiskClick
  DIM d AS INTEGER
  d = ExtBarDoc
  IF d < 0 THEN EXIT SUB
  ExtUseDisk d
END SUB

SUB ExtUseDisk(d AS INTEGER)
  ExtBusy = 1
  ExtReload d, 1
  ExtBusy = 0
  SetState RunState, TR$("Loaded the disk's version of {file}: yours is in the local history, and Undo brings it back", "file", FileTitle(DocFile(d)))
  Application.Announce TR$("Loaded the disk's version. Undo brings yours back.")
  AnalyzeActive
  ExtShowBar
  FocusCodeSoon d
END SUB

' Keep Mine: the question goes; the next Save asks before it replaces the
' disk's version.
SUB ExtMineClick
  DIM d AS INTEGER
  d = ExtBarDoc
  IF d < 0 THEN EXIT SUB
  ExtKeepMine d
END SUB

SUB ExtKeepMine(d AS INTEGER)
  DocExtAsk(d) = 0
  SetState RunState, TR$("Keeping your version of {file}: saving it asks first", "file", FileTitle(DocFile(d)))
  Application.Announce TR$("Keeping your version. Saving it asks first.")
  ExtShowBar
  FocusCodeSoon d
END SUB

' Show: the files of the last batch, in the Output.
SUB ExtShowClick
  DIM i AS INTEGER, n AS INTEGER
  n = TALLY(ExtNoteFiles, CHR$(10))
  OutputLine TRN$("{n} file was updated outside Studio:", "{n} files were updated outside Studio:", n)
  FOR i = 1 TO n
    OutputLine "  " + ProjectRel(FIELD$(ExtNoteFiles, CHR$(10), i))
  NEXT
  OutputBox.Page = "output"
  Dock.FocusPane("outputbox")
  ExtNoteOn = 0
  ExtShowBar
END SUB

SUB ExtCloseClick
  ExtNoteOn = 0
  ExtShowBar
  IF ActiveCode >= 0 THEN FocusCodeSoon ActiveCode
END SUB

' ---- Studio's hooks ------------------------------------------------------------------------------

' (documents.inc: a document opened — its file is Studio's as it was read)
SUB ExtOpened(d AS INTEGER)
  DocExtAsk(d) = 0
  DocGone(d) = 0
  Proj.Watch DocFile(d)
END SUB

' (a document closed: no longer watched; its strip goes)
SUB ExtClosed(d AS INTEGER)
  IF d < 0 THEN EXIT SUB
  Proj.Unwatch DocFile(d)
  DocExtAsk(d) = 0
  DocGone(d) = 0
  IF ExtBarDoc = d THEN
    Dock.DocumentBar(DocPane(d), "")
    ExtBarDoc = -1
    ExtBar.Visible = 0
  END IF
END SUB

' (Studio wrote Path itself: its own, never a change from outside)
SUB ExtOwnWrite(Path AS STRING)
  Proj.Watch Path
END SUB

' Before Save writes document d: its file changed (or was deleted) outside
' Studio since Studio read or wrote it — asked first, Cancel the default.
' 1 to save.
FUNCTION ExtSaveOk(d AS INTEGER) AS INTEGER
  DIM path AS STRING, yes AS INTEGER
  Result = 1
  path = DocFile(d)
  IF Proj.ChangedOnDisk(path) = 0 THEN EXIT FUNCTION
  ExtBusy = 1
  IF FILEEXISTS(path) = 0 THEN
    ConsentBegin TR$("Save {file} again?", "file", FileTitle(path)), TR$("{file} was deleted outside Studio. Saving makes the file again with the text you have here.", "file", FileTitle(path))
    yes = ConsentAsk("", "", TR$("&Save Again"), TR$("Cancel"))
  ELSE
    ConsentBegin TR$("Save {file} anyway?", "file", FileTitle(path)), TR$("{file} changed outside Studio since you opened it. Saving replaces the version on disk with yours; Studio keeps the disk's version in the local history first.", "file", FileTitle(path))
    yes = ConsentAsk("", "", TR$("&Save Anyway"), TR$("Cancel"))
  END IF
  ExtBusy = 0
  IF yes = 0 THEN
    Result = 0
    SetState RunState, TR$("Not saved: {file} changed outside Studio", "file", FileTitle(path))
    EXIT FUNCTION
  END IF
  ' (the disk's version kept before it is replaced; it may be the only copy)
  IF ProjectOpen AND FILEEXISTS(path) THEN
    IF Repo.Folder = "" THEN Repo.Folder = Proj.Folder
    Repo.Snapshot path, "on-disk"
  END IF
END FUNCTION

' (documents.inc's SaveDocument, after the write: Studio's own)
SUB ExtSaved(d AS INTEGER)
  Proj.Watch DocFile(d)
  DocExtAsk(d) = 0
  IF DocGone(d) THEN
    DocGone(d) = 0
  END IF
  IF ExtBarDoc = d THEN ExtShowBar
END SUB

' (project.inc: a project opened — its file watched; closed — nothing)
SUB ExtProjectOpened
  Proj.UnwatchAll
  IF ProjectFile <> "" AND FILEEXISTS(ProjectFile) THEN Proj.Watch ProjectFile
  ExtNoteOn = 0
END SUB

SUB ExtProjectClosed
  Proj.UnwatchAll
  ExtNoteOn = 0
  ExtBarDoc = -1
  ExtBar.Visible = 0
END SUB

' File > Reopen Project: the project closed (changes asked about first, as
' when Studio closes) and opened again from the disk.
SUB ReopenProject
  DIM p AS STRING
  IF ProjectOpen = 0 THEN EXIT SUB
  p = ProjectFile
  IF AskToSaveAll = 0 THEN EXIT SUB
  OpenProject p
  OutputLine TR$("Opened {file} again from the disk", "file", FileTitle(p))
  SetState "stopped", TR$("Opened {file} again from the disk", "file", FileTitle(p))
END SUB

' The commands (commands.inc's table): 1 when Id was one of them.
FUNCTION ExternalCommand(Id AS STRING) AS INTEGER
  DIM d AS INTEGER
  Result = 1
  d = ActiveCode
  SELECT CASE Id
    CASE "file.reopenProject": ReopenProject
    CASE "file.compareDisk": IF d >= 0 THEN ExtCompareDoc d
    CASE "file.useDisk": IF d >= 0 THEN ExtUseDisk d
    CASE "file.keepMine": IF d >= 0 THEN ExtKeepMine d
    CASE ELSE: Result = 0
  END SELECT
END FUNCTION

' (a test step: ext:<what>=<value>)
'   ext:check           a look now, as when Studio comes to the front
'   ext:go=<n>          tells the test's outside writer to make change n
'                       (a file go-<n> in RAPIDR_TEST_EXT's folder)…
'   ext:await=<n>       …and waits until it says it is done (done-<n>)
'   ext:openall         every file of the project opened
'   ext:report          what Studio shows, in the Output: [ext] …
SUB ExternalStep(S AS STRING)
  DIM k AS INTEGER, what AS STRING, v AS STRING, folder AS STRING, i AS INTEGER, d AS INTEGER, t AS STRING
  k = INSTR(S, "=")
  IF k > 0 THEN
    what = LEFT$(S, k - 1): v = MID$(S, k + 1)
  ELSE
    what = S
  END IF
  folder = ENVIRON$("RAPIDR_TEST_EXT")
  SELECT CASE what
    CASE "check": ExtCheck
    CASE "go"
      IF folder <> "" THEN
        ExtMarker.Open(folder + "/go-" + v, 65535)
        ExtMarker.WriteStr("go", 2)
        ExtMarker.Close
      END IF
    CASE "await"
      ' (asked again at the next step's turn until the writer is done: a
      ' minute at most)
      IF folder <> "" AND FILEEXISTS(folder + "/done-" + v) = 0 THEN
        ExtTicks = 0
        IF INSTR(DoList, "ext:await=" + v) = 0 THEN DoList = "ext:await=" + v + "," + DoList
      END IF
    CASE "open"
      ' (ext:open=<a file of the project>: opened, in front)
      OpenDocument Proj.Folder + "/" + v
    CASE "openall"
      FOR i = 0 TO Proj.FileCount - 1
        OpenQuietly = 1
        OpenDocument Proj.FullPath(i)
        OpenQuietly = 0
      NEXT
      Dock.ActiveDocument = DocPane(DocOf(ProjectMain))
    CASE "report"
      t = "[ext] status=" + Status.Panel(0).Caption + " bar=" + ExtBarMode
      FOR d = 0 TO DocCount - 1
        IF DocOpen(d) THEN t = t + " | " + FileTitle(DocFile(d)) + ":" + Dock.PaneTitle(DocPane(d)) + ":dirty=" + Num(DocDirty(d)) + ":ask=" + Num(DocExtAsk(d)) + ":gone=" + Num(DocGone(d))
      NEXT
      IF ActiveCode >= 0 THEN
        IF ProjectOpen THEN t = t + " | history=" + REPLACESUBSTR$(Repo.LocalHistory(DocFile(ActiveCode)), CHR$(10), ";")
      END IF
      OutputLine t
  END SELECT
END SUB

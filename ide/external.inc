' external.inc: files changed outside Studio (lane EXTERNAL-CHANGES;
' Preview 5: "Never lose anything"). Another AI agent, an editor, git
' checking out a branch: a file Studio has open changes on disk.
'
'   - Studio looks (RPROJECT's ExternalChanges: the time, size and a hash of
'     each open file and of the project file) when its window is in front
'     again, and every two seconds while it is;
'   - a file with no unsaved changes here is loaded again quietly: what the
'     editor had is kept in the local history first ("before reload"), and
'     stays in the editor's Undo (one Ctrl+Z brings it back); a toast says
'     so — "main.rr was reloaded" —, with Show (notify.inc); the status bar
'     does not say it again, the history keeps it;
'   - a file with unsaved changes here gets a question as one slim line on
'     THAT editor only (the design guide's decision strip: RNotifyBar, in
'     RDockManager's DocumentBar above the editor and the designer, its
'     4-pixel mark in the warning colour): Compare (the disk's version
'     beside yours), Use Disk Version (yours kept in the local history
'     first; Undo still brings it back), Keep Mine;
'   - Save never overwrites newer content on disk silently: it asks first
'     ("main.rr changed outside Studio since you opened it. Save anyway?",
'     Cancel the default);
'   - a file deleted outside keeps its tab, marked "(deleted on disk)", and
'     its text; Save makes it again (asked first);
'   - the project file changed (an agent added or removed files): the
'     Project pane follows quietly;
'   - many files at once (git checkout, an agent rewriting 20 files): one
'     toast naming them — "5 files were reloaded: main.rr, form1.rr,
'     utils.inc +2" —, with Show (the list, in a searchable card);
'   - Studio's own writes (Save, RAI's Apply, an AI app's changes, Build,
'     Versions' Go Back) never count: Studio watches what it wrote.
' Studio on the web: files live in the page's store and change only
' through Studio — the checks find nothing there.
' File > Reopen Project closes the project and opens it fresh.

DECLARE SUB ExtTick
DECLARE SUB ExtChoose(Index AS INTEGER, Caption AS STRING)
DECLARE SUB ExtEscape
DECLARE SUB ExtBarEntered
DECLARE SUB ExtBarLeft
DECLARE SUB ExtShowBar
DECLARE SUB ExtReload(d AS INTEGER, Asked AS INTEGER)
DECLARE SUB ExtProjectFile(What AS STRING)
DECLARE FUNCTION ExtToastAction(Key AS STRING, Index AS INTEGER) AS INTEGER

' (Studio was in the background at the last tick: a look once it is back)
DIM ExtAway AS INTEGER
DIM ExtTicks AS INTEGER
' (a look, a question, a reload going on: the timer waits)
DIM ExtBusy AS INTEGER
' (DocExtAsk, DocGone: decl.inc)
' (the strip: on which document, and the document just asked about — its
' question is said to a screen reader once, as it appears, not each time
' the tab comes to the front)
DIM ExtBarDoc AS INTEGER
DIM ExtBarMode AS STRING
DIM ExtAskNew AS INTEGER
' (the keyboard is in the strip: Next Area reaches it, then goes on)
DIM ExtBarFocused AS INTEGER
' (the last batch of files updated quietly: their paths, a line each — what
' the toast's Show lists)
DIM ExtNoteFiles AS STRING
' (tests: the folder an outside writer and Studio meet in — RAPIDR_TEST_EXT)
DIM ExtMarker AS RFileStream

CREATE ExtTimer AS RTimer
  Interval = 500
  Enabled = 0
  OnTimer = ExtTick
END CREATE

' The question on a document: one slim line above its views (RDOCKMANAGER's
' DocumentBar), RNotifyBar's.
CREATE ExtBar AS RNotifyBar
  Align = 0
  OnChoose = ExtChoose
  OnEscape = ExtEscape
  OnEnter = ExtBarEntered
  OnExit = ExtBarLeft
END CREATE

SUB SetUpExternal
  ExtBarDoc = -1
  ExtAskNew = -1
  ExtTimer.Enabled = 1
END SUB

SUB ApplyExternalChrome
  DIM d AS INTEGER
  ExtBar.Font = UIFont
  ' (a deleted file's tab says so: its words in the language now)
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) AND DocGone(d) THEN DocTitles d
  NEXT
  ExtShowBar
END SUB

' The strip shows what the document in front needs: its question when its
' file changed on disk under unsaved changes, else nothing.
SUB ExtShowBar
  DIM d AS INTEGER, mode AS STRING, t AS STRING, b AS STRING
  d = ActiveCode
  IF d >= 0 THEN
    IF DocExtAsk(d) THEN mode = "ask"
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
    ExtBar.Hide
    EXIT SUB
  END IF
  t = TR$("{file} changed on disk while you were editing it.", "file", FileTitle(DocFile(d)))
  b = TR$("&Compare") + "," + TR$("&Use Disk Version") + "," + TR$("&Keep Mine")
  IF ExtAskNew = d THEN
    ' (new: said to a screen reader at once, once)
    ExtAskNew = -1
    ExtBar.Ask t, b, "warning"
  ELSE
    ExtBar.Text = t
    ExtBar.Buttons = b
    ExtBar.Kind = "warning"
    ExtBar.Visible = 1
  END IF
  ExtBar.AccessibleDescription = TR$("Compare shows the disk's version beside yours. Use Disk Version loads it: yours is kept in the local history, and Undo brings it back. Keep Mine keeps yours: saving it asks before it replaces the disk's version.")
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
  DIM done AS STRING, doneN AS INTEGER, asked AS INTEGER, gone AS INTEGER, said AS STRING, k AS INTEGER
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
        k = Toast(said, "warning", "", "gone")
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
          ' (unsaved changes here: asked, never overwritten — the strip on
          ' this editor says it, once to a screen reader)
          IF DocExtAsk(d) = 0 THEN ExtAskNew = d
          DocExtAsk(d) = 1
          asked = asked + 1
          OutputLine TR$("{file} changed outside Studio, and you have unsaved changes in it", "file", FileTitle(path))
        ELSE
          ExtReload d, 0
          done = done + path + CHR$(10)
          doneN = doneN + 1
        END IF
      END IF
    END IF
  NEXT
  ' (a toast, once, naming them: the status bar doesn't say it again; the
  ' Output keeps the log; the bell's history keeps the toast)
  IF doneN = 1 THEN
    OutputLine TR$("{file} was updated outside Studio", "file", FileTitle(FIELD$(done, CHR$(10), 1)))
    ExtNoteFiles = done
    k = Toast(TR$("{file} was reloaded", "file", FileTitle(FIELD$(done, CHR$(10), 1))), "info", TR$("Show"), "reload")
  ELSEIF doneN > 1 THEN
    OutputLine TRN$("{n} file was updated outside Studio", "{n} files were updated outside Studio", doneN)
    ExtNoteFiles = done
    k = Toast(TRN$("{n} file was reloaded: {names}", "{n} files were reloaded: {names}", doneN, "names", NotifyNames(done)), "info", TR$("Show"), "reload")
  END IF
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

' The strip's buttons, 1 Compare, 2 Use Disk Version, 3 Keep Mine (the
' strip has gone as the answer was given; Compare leaves the question asked).
SUB ExtChoose(Index AS INTEGER, Caption AS STRING)
  DIM d AS INTEGER
  d = ExtBarDoc
  IF d < 0 THEN EXIT SUB
  SELECT CASE Index
    CASE 1
      ' (Compare: the disk's version beside the editor's, in the What
      ' Changed document — versions.inc; the question stays)
      ExtCompareDoc d
      ExtShowBar
    CASE 2: ExtUseDisk d
    CASE 3: ExtKeepMine d
  END SELECT
END SUB

' Escape in the strip: the keyboard goes back to the editor; the question
' stays asked.
SUB ExtEscape
  IF ActiveCode >= 0 THEN FocusCodeSoon ActiveCode
END SUB

SUB ExtBarEntered
  ExtBarFocused = 1
END SUB

SUB ExtBarLeft
  ExtBarFocused = 0
END SUB

SUB ExtCompareDoc(d AS INTEGER)
  VerDiffFile = DocFile(d)
  VerShowDiff "disk"
END SUB

' Use Disk Version: the editor's text kept in the local history, then the
' disk's loaded (Undo still brings it back).
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
SUB ExtKeepMine(d AS INTEGER)
  DocExtAsk(d) = 0
  SetState RunState, TR$("Keeping your version of {file}: saving it asks first", "file", FileTitle(DocFile(d)))
  Application.Announce TR$("Keeping your version. Saving it asks first.")
  ExtShowBar
  FocusCodeSoon d
END SUB

' A toast's action (notify.inc): Show — the file reloaded, or the list of
' the files of the last batch in a searchable card. 1 when it was ours.
FUNCTION ExtToastAction(Key AS STRING, Index AS INTEGER) AS INTEGER
  IF Key <> "reload" THEN EXIT FUNCTION
  Result = 1
  IF TALLY(ExtNoteFiles, CHR$(10)) <= 1 THEN
    IF ExtNoteFiles <> "" THEN OpenDocument FIELD$(ExtNoteFiles, CHR$(10), 1)
  ELSE
    ShowFileList ExtNoteFiles
  END IF
END FUNCTION

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
    ExtBar.Hide
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
END SUB

SUB ExtProjectClosed
  Proj.UnwatchAll
  ExtBarDoc = -1
  ExtBar.Hide
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
    CASE "choose"
      ' (ext:choose=<n>: the strip's button n, as a click on it would)
      ExtBar.Press(VAL(v))
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

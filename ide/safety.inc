' safety.inc: Studio keeps your work.
'
' Something going wrong in Studio's own code never ends Studio: a run-time
' error ends the handler it happened in (the window keeps answering, as any
' RapidR program's does) and Application.OnError (StudioFailed) hears it.
' Studio then
'   - keeps a copy of every document with unsaved changes at once,
'   - writes the error to its log (RPROJECT.LogFile) with Studio's file and
'     line, and says it in the Output,
'   - says it in a dialog: "Something went wrong in Studio … Your work is
'     safe" (OK, Copy Details); errors that come while it shows are counted
'     in it, not shown again.
' The copies are also kept as you type (every 20 seconds while there are new
' changes; RPROJECT's Recovery…: Studio's own folder, never the project's),
' so even a Studio that is killed — or a computer switched off — offers
' "Restore your unsaved work?" at the next start: Restore puts each change
' back in its document (not saved: one Undo takes it back to the file),
' Discard lets the copies go, Not Now keeps them for the next start. A
' Studio closed as it should lets them go.

DECLARE SUB StudioFailed(Message AS STRING, File AS STRING, Line AS INTEGER)
DECLARE SUB FailPick(Sender AS RButton)
DECLARE SUB KeepTick
DECLARE SUB OfferTick
DECLARE SUB KeepAll(All AS INTEGER)
DECLARE SUB LayOutFail
DECLARE SUB RestoreFound(Found AS STRING)
DECLARE FUNCTION ShortPaths(Text AS STRING) AS STRING

DIM FailCount AS INTEGER     ' (the errors this Studio has had)
DIM FailShowing AS INTEGER   ' (the dialog is up: more errors are counted in it)
DIM FailMore AS INTEGER      ' (errors since it showed)
DIM FailDetails AS STRING    ' (what Copy Details copies)
DIM FailIconBmp AS RBitmap
DIM FailIconLoaded AS INTEGER
DIM KeepChanged(MAXDOCS) AS INTEGER ' (changed since its copy was kept)
DIM KeepTimer AS RTimer
DIM OfferTimer AS RTimer
DIM Restoring AS INTEGER     ' (a test step answers the question: 1 Restore, 2 Discard, 3 Not Now)

CREATE FailNote AS RForm
  Caption = "RapidR Studio"
  Width = 520: Height = 220
  BorderStyle = 3
  CREATE FailOK AS RButton
    Caption = "OK": Tag = 1
    Default = 1: Cancel = 1
    OnClick = FailPick
  END CREATE
  CREATE FailCopy AS RButton
    Caption = "&Copy Details": Tag = 2
    OnClick = FailPick
  END CREATE
  CREATE FailIcon AS RImage
    Left = 20: Top = 20: Width = 56: Height = 56
    Stretch = 1
    AccessibleName = "RapidR Studio"
  END CREATE
  CREATE FailTitle AS RLabel
    WordWrap = 1: AutoSize = 0
  END CREATE
  CREATE FailText AS RLabel
    WordWrap = 1: AutoSize = 0
  END CREATE
END CREATE

SUB SetUpSafety
  ' (from the very start: an error anywhere after this is told)
  Application.OnError = StudioFailed
  KeepTimer.Interval = 20000
  KeepTimer.Enabled = 1
  KeepTimer.OnTimer = KeepTick
  OfferTimer.Interval = 300
  OfferTimer.Enabled = 0
  OfferTimer.OnTimer = OfferTick
END SUB

' ---- something went wrong in Studio's own code --------------------------------------

SUB StudioFailed(Message AS STRING, File AS STRING, Line AS INTEGER)
  DIM where AS STRING, said AS STRING
  FailCount = FailCount + 1
  where = ""
  IF File <> "" THEN where = File
  IF Line > 0 THEN where = where + " line " + LTRIM$(STR$(Line))
  where = LTRIM$(where)
  ' (first the work: every unsaved change kept, whatever happens next)
  KeepAll 1
  said = "Something went wrong in Studio: " + Message
  IF where <> "" THEN said = said + " (" + where + ")"
  FailDetails = "RapidR Studio (RapidR " + Lang.Version + "): " + Message
  IF where <> "" THEN FailDetails = FailDetails + " (at Studio's " + where + ")"
  Proj.LogError DATE$ + " " + TIME$ + "  " + FailDetails
  OutputLine said + ". Your work is safe; Studio goes on."
  IF FailShowing THEN
    ' (one dialog at a time: the one up says how many came since)
    FailMore = FailMore + 1
    FailTitle.Caption = "Something went wrong in Studio (" + LTRIM$(STR$(FailMore + 1)) + " times)"
    EXIT SUB
  END IF
  FailMore = 0
  LayOutFail
  FailTitle.Caption = "Something went wrong in Studio"
  FailText.Caption = ShortPaths(RTRIM$(Message)) + IIF(where <> "", " (" + where + ")", "") + CHR$(10) + CHR$(10) + "Your work is safe: nothing was lost, and Studio goes on. A copy of your unsaved changes is kept. Copy Details copies what happened; Studio's log keeps it too."
  FailText.Hint = Proj.LogFile
  Application.Announce "Something went wrong in Studio. Your work is safe.", 1
  FailShowing = 1
  FailNote.Center
  FailNote.ShowModal
  FailShowing = 0
END SUB

' (a long path in a message — a word a line can't break — as its last
' two parts: …/studio/no-such-folder/x.txt)
FUNCTION ShortPaths(Text AS STRING) AS STRING
  DIM i AS INTEGER, n AS INTEGER, w AS STRING, k AS INTEGER, cut AS INTEGER
  n = TALLY(Text, " ") + 1
  FOR i = 1 TO n
    w = FIELD$(Text, " ", i)
    IF LEN(w) > 40 AND INSTR(w, "/") > 0 THEN
      ' (from the third / from its end)
      cut = 0
      FOR k = LEN(w) TO 1 STEP -1
        IF MID$(w, k, 1) = "/" THEN
          cut = cut + 1
          IF cut = 3 THEN
            w = "…" + MID$(w, k)
            EXIT FOR
          END IF
        END IF
      NEXT
    END IF
    IF i > 1 THEN Result = Result + " "
    Result = Result + w
  NEXT
END FUNCTION

' (a tidy look, as the save question's: the classic look's sizes or RapidR's)
SUB LayOutFail
  DIM pad AS INTEGER, ic AS INTEGER, x AS INTEGER, w AS INTEGER, y AS INTEGER, bh AS INTEGER, gap AS INTEGER
  IF ThemeName = "classic" THEN
    pad = 14: ic = 32: bh = 23: gap = 6
  ELSE
    pad = 20: ic = 56: bh = 32: gap = 8
  END IF
  FailNote.Font.Name = UIFont.Name: FailNote.Font.Size = UIFont.Size
  FailTitle.Font = SubFont
  FailText.Font.Color = TC("page.dim")
  IF FailIconLoaded = 0 THEN
    FailIconBmp.LoadFromFile HomeDir + "ide/assets/rapidr-studio.svg"
    FailIcon.BMPHandle = FailIconBmp
    FailIconLoaded = 1
  END IF
  x = pad + ic + 16
  w = 420
  FailIcon.Left = pad: FailIcon.Top = pad: FailIcon.Width = ic: FailIcon.Height = ic
  FailTitle.Left = x: FailTitle.Top = pad: FailTitle.Width = w: FailTitle.Height = 26
  FailText.Left = x: FailText.Top = pad + 32: FailText.Width = w: FailText.Height = 128
  y = pad + 32 + 128 + gap
  FailOK.Width = 96: FailOK.Height = bh: FailOK.Top = y: FailOK.Left = x + w - 96
  FailCopy.Width = 130: FailCopy.Height = bh: FailCopy.Top = y: FailCopy.Left = FailOK.Left - gap - 130
  FailNote.ClientWidth = x + w + pad
  FailNote.ClientHeight = y + bh + pad
END SUB

SUB FailPick(Sender AS RButton)
  IF Sender.Tag = 2 THEN
    Clipboard.Text = FailDetails
    FailCopy.Caption = "Copied"
    EXIT SUB
  END IF
  FailCopy.Caption = "&Copy Details"
  FailNote.Close
END SUB

' The tests' steps (--do): safety:fail (a run-time error in Studio's own
' code: a file written into a folder that isn't there; safety:divide, a
' division by zero), safety:keep (the
' copies kept now, not at the next tick), safety:restore, safety:discard,
' safety:later (the question at the start answered so).
FUNCTION SafetyStep(Stp AS STRING) AS INTEGER
  DIM f AS RFileStream
  IF LEFT$(Stp, 7) <> "safety:" THEN EXIT FUNCTION
  Result = 1
  SELECT CASE MID$(Stp, 8)
    CASE "fail"
      f.Open(Proj.StudioFolder + "/no-such-folder/studio-test.txt", 65535)
      f.Close
    CASE "divide"
      ' (an error of the language's own: the page's store has no folders
      ' to be missing)
      Restoring = Restoring \ Restoring
    CASE "keep"
      KeepAll 0
    CASE "restore", "discard", "later"
      Restoring = INSTR("restore discard later", MID$(Stp, 8)) \ 8 + 1
      OfferTick
      Restoring = 0
  END SELECT
END FUNCTION

' ---- the copies of unsaved work ------------------------------------------------------

' (a document's text changed: its copy is kept at the next tick)
SUB KeepNoted(d AS INTEGER)
  IF d >= 0 AND d <= MAXDOCS THEN KeepChanged(d) = 1
END SUB

SUB KeepTick
  KeepAll 0
END SUB

' Keeps a copy of each document with unsaved changes (All: every one, else
' those changed since their copy), and lets go of the copies of those saved.
SUB KeepAll(All AS INTEGER)
  DIM d AS INTEGER
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) AND DocDirty(d) THEN
      IF All OR KeepChanged(d) THEN
        IF Proj.RecoveryKeep(DocFile(d), DocFile(d), FileTitle(DocFile(d)), CodeDoc(d).Text) THEN KeepChanged(d) = 0
      END IF
    ELSEIF KeepChanged(d) THEN
      Proj.RecoveryForget DocFile(d)
      KeepChanged(d) = 0
    END IF
  NEXT
END SUB

' (a document saved, or closed with its changes let go: no copy of it)
SUB KeepForget(d AS INTEGER)
  IF d < 0 THEN EXIT SUB
  Proj.RecoveryForget DocFile(d)
  KeepChanged(d) = 0
END SUB

' (Studio closed as it should: every change saved or let go)
SUB KeepEnd
  KeepTimer.Enabled = 0
  Proj.RecoveryEnd
END SUB

' ---- restoring them at the next start --------------------------------------------------

' (once Studio's window is up)
SUB OfferRestoreSoon
  IF Proj.RecoveryFound <> "" THEN OfferTimer.Enabled = 1
END SUB

SUB OfferTick
  DIM found AS STRING, n AS INTEGER, i AS INTEGER, names AS STRING, row AS STRING, answer AS INTEGER
  OfferTimer.Enabled = 0
  found = Proj.RecoveryFound
  IF found = "" THEN EXIT SUB
  n = TALLY(found, CHR$(10))
  FOR i = 1 TO n
    row = FIELD$(found, CHR$(10), i)
    IF i <= 3 THEN
      IF names <> "" THEN names = names + ", "
      names = names + FIELD$(row, CHR$(9), 4)
    END IF
  NEXT
  IF n > 3 THEN names = names + " and " + LTRIM$(STR$(n - 3)) + " more"
  PromptRestore = 1
  IF Restoring THEN
    ' (a test's step: 1 Restore, 2 Discard, 3 Not Now)
    answer = Restoring MOD 3
  ELSE
    answer = AskSave("Restore your unsaved work?", "Studio closed before " + IIF(n = 1, "a file was", LTRIM$(STR$(n)) + " files were") + " saved: " + names + ". Restore puts your changes back in " + IIF(n = 1, "its editor", "their editors") + " (not saved yet; Undo goes back to the file).", 0)
  END IF
  PromptRestore = 0
  IF answer = PROMPT_SAVE THEN
    RestoreFound found
  ELSEIF answer = PROMPT_DONT THEN
    Proj.RecoveryDiscard ""
    OutputLine "The unsaved work Studio kept was let go."
  ELSE
    OutputLine "Studio keeps your unsaved work: it asks again at the next start."
  END IF
END SUB

' Each copy back in its document: the file opened (with its project when
' none is open), its text the copy's — one undo step, so Undo goes back to
' the file —, marked changed (not saved). A file that is gone isn't made
' again: its copy is kept, and asked about at the next start.
SUB RestoreFound(Found AS STRING)
  DIM n AS INTEGER, i AS INTEGER, row AS STRING, run AS STRING, key AS STRING, path AS STRING, text AS STRING, d AS INTEGER, done AS INTEGER, missed AS INTEGER, patch AS STRING
  n = TALLY(Found, CHR$(10))
  FOR i = 1 TO n
    row = FIELD$(Found, CHR$(10), i)
    run = FIELD$(row, CHR$(9), 1)
    key = FIELD$(row, CHR$(9), 2)
    path = FIELD$(row, CHR$(9), 3)
    text = Proj.RecoveryText(run, key)
    d = -1
    IF path <> "" AND FILEEXISTS(path) THEN
      ' (its project first, when none is open: the project's tree, its Run)
      IF ProjectOpen = 0 THEN OpenProject path
      d = DocOf(path)
      IF d < 0 THEN
        OpenDocument path
        d = DocOf(path)
      END IF
    END IF
    IF d >= 0 THEN
      patch = AITools.Patch(CodeDoc(d).Text, text)
      IF patch <> "" THEN
        IF CodeDoc(d).ApplyPatches(patch, 0) THEN
          DirtyFollows d
          KeepNoted d
        END IF
      END IF
      done = done + 1
    ELSE
      missed = missed + 1
      OutputLine "Couldn't restore " + IIF(path <> "", path + " (it isn't there any more)", FIELD$(row, CHR$(9), 4)) + ": its copy is kept for the next start."
    END IF
  NEXT
  IF missed = 0 THEN Proj.RecoveryDiscard ""
  ' (kept again at once: this Studio's own copies now)
  KeepAll 1
  OutputLine "Restored the unsaved work of " + LTRIM$(STR$(done)) + IIF(done = 1, " file", " files") + " (not saved yet)."
  UpdateCommands
END SUB

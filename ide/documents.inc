' documents.inc: source files as documents — a code editor each, and a
' designer for the form a file creates — and what the side panes show of
' the active one: its outline (RLanguageService), its problems, its form's
' properties.

DIM OutlineLine(400) AS INTEGER
DIM OutlineCount AS INTEGER
' (the outline shown: its document and its text — the same again after a
' pause, nothing is done: FillOutline)
DIM OutlineShownDoc AS INTEGER
DIM OutlineShownText AS STRING
' (the problems shown: their text and whose — the same again, the list
' stays as it is: FillProblemsOf; ClearProblems forgets them)
DIM ProblemsShownText AS STRING
DIM ProblemsShownPath AS STRING
' (SymbolIcons' icons by name, each added once a fill: SymbolIcon)
DIM SymbolIconName(80) AS STRING
DIM SymbolIconAt(80) AS INTEGER
DIM SymbolIconKeys AS INTEGER

' (a path's file name)
FUNCTION FileTitle(Path AS STRING) AS STRING
  DIM p AS STRING, i AS INTEGER
  p = REPLACESUBSTR$(Path, "\", "/")
  i = RINSTR(p, "/")
  IF i > 0 THEN Result = MID$(p, i + 1) ELSE Result = p
END FUNCTION

' (a file name without its extension)
FUNCTION BaseName(Path AS STRING) AS STRING
  DIM t AS STRING, i AS INTEGER
  t = FileTitle(Path)
  i = RINSTR(t, ".")
  IF i > 1 THEN Result = LEFT$(t, i - 1) ELSE Result = t
END FUNCTION

' (a file's extension, lower case)
FUNCTION FileExt(Path AS STRING) AS STRING
  DIM i AS INTEGER
  i = RINSTR(Path, ".")
  IF i > 0 THEN Result = LCASE$(MID$(Path, i + 1)) ELSE Result = ""
END FUNCTION

' (a BASIC source — RapidR's .rr, RapidQ's .bas / .rqw / .rqb / .rq, an
' .inc: the language service reads these; a report or a data file isn't)
FUNCTION IsBasicFile(Path AS STRING) AS INTEGER
  SELECT CASE FileExt(Path)
    CASE "rr", "bas", "inc", "rqw", "rqb", "rq": Result = 1
  END SELECT
END FUNCTION

' (a string literal's text, quotes off)
FUNCTION Unquote(v AS STRING) AS STRING
  IF LEFT$(v, 1) = CHR$(34) AND RIGHT$(v, 1) = CHR$(34) AND LEN(v) >= 2 THEN Result = MID$(v, 2, LEN(v) - 2) ELSE Result = v
END FUNCTION

' (S-PANELS: the stand-in scanner of the form's properties is gone: the
' inspector follows the designer, which reads the code — designer.inc)

' ---- the outline (RLanguageService) ------------------------------------------------

' (an icon of SymbolIcons by its name: added once a fill — a big file's
' thousands of items share a few, each drawn once, not once an item)
FUNCTION SymbolIcon(Name AS STRING) AS INTEGER
  DIM i AS INTEGER
  FOR i = 0 TO SymbolIconKeys - 1
    IF SymbolIconName(i) = Name THEN
      Result = SymbolIconAt(i)
      EXIT FUNCTION
    END IF
  NEXT
  SymbolIcons.AddIcon Name
  Result = SymbolIcons.Count - 1
  IF SymbolIconKeys <= 80 THEN
    SymbolIconName(SymbolIconKeys) = Name
    SymbolIconAt(SymbolIconKeys) = Result
    SymbolIconKeys = SymbolIconKeys + 1
  END IF
END FUNCTION

' (an outline kind's icon in SymbolIcons)
FUNCTION OutlineIcon(Kind AS STRING, Detail AS STRING) AS INTEGER
  DIM t AS STRING
  IF Kind = "component" THEN
    t = UCASE$(Detail)
    IF INSTR(t, " ") > 0 THEN t = MID$(t, RINSTR(t, " ") + 1)
    IF t <> "" THEN
      Result = SymbolIcon(t)
      EXIT FUNCTION
    END IF
  END IF
  Result = SymbolIcon("symbols/" + Kind)
END FUNCTION

SUB FillOutline(d AS INTEGER)
  DIM text AS STRING, n AS INTEGER, i AS INTEGER, row AS STRING
  DIM depth AS INTEGER, kind AS STRING, nm AS STRING, detail AS STRING, ln AS INTEGER, img AS INTEGER
  DIM rowAt AS INTEGER, rowEnd AS INTEGER
  DIM parentNode(20) AS INTEGER
  IF d >= 0 THEN
    text = Lang.Outline(DocFile(d))
    ' (the outline shown already — typing inside a SUB, a letter typed and
    ' taken back —: nothing to do; a big file's thousands of items aren't
    ' added again after every pause, and what was folded stays folded)
    IF d = OutlineShownDoc AND text = OutlineShownText AND OutlineTree.ItemCount = OutlineCount THEN EXIT SUB
  END IF
  OutlineTree.Clear
  SymbolIcons.Clear
  SymbolIconKeys = 0
  OutlineCount = 0
  OutlineShownDoc = -1
  OutlineShownText = ""
  IF d < 0 THEN EXIT SUB
  n = TALLY(text, CHR$(10))
  ' (each row where the last one ended: FIELD$ for row i looked through
  ' every row before it — a big include's thousands of rows took seconds)
  rowAt = 1
  FOR i = 1 TO n
    rowEnd = INSTR(rowAt, text, CHR$(10))
    row = MID$(text, rowAt, rowEnd - rowAt)
    rowAt = rowEnd + 1
    depth = VAL(FIELD$(row, CHR$(9), 1))
    kind = FIELD$(row, CHR$(9), 2)
    nm = FIELD$(row, CHR$(9), 3)
    detail = FIELD$(row, CHR$(9), 4)
    ln = VAL(FIELD$(row, CHR$(9), 5))
    IF depth > 19 THEN depth = 19
    IF kind = "component" AND detail <> "" THEN nm = nm + ": " + detail
    IF depth = 0 THEN
      OutlineTree.AddItems nm
    ELSE
      OutlineTree.AddChildItems parentNode(depth - 1), nm
    END IF
    img = OutlineIcon(kind, detail)
    OutlineTree.Item(OutlineTree.ItemCount - 1).ImageIndex = img
    OutlineTree.Item(OutlineTree.ItemCount - 1).SelectedIndex = img
    parentNode(depth) = OutlineTree.ItemCount - 1
    IF OutlineTree.ItemCount <= 400 THEN OutlineLine(OutlineTree.ItemCount - 1) = ln - 1
  NEXT
  OutlineCount = OutlineTree.ItemCount
  OutlineTree.FullExpand
  OutlineShownDoc = d
  OutlineShownText = text
END SUB

' ---- problems (RLanguageService's diagnostics: the compiler's messages) ----------

SUB FillProblems(d AS INTEGER)
  IF d < 0 THEN
    ClearProblems
    EXIT SUB
  END IF
  ' (a file of the open project: the whole project's problems — S5)
  IF InProject(DocFile(d)) THEN FillProblemsOf ProjectMain ELSE FillProblemsOf DocFile(d)
END SUB

' (whether Path is a file of the open project: its main file, or one the
' project lists)
FUNCTION InProject(Path AS STRING) AS INTEGER
  DIM i AS INTEGER, p AS STRING
  IF ProjectOpen = 0 OR ProjectMain = "" THEN EXIT FUNCTION
  p = LCASE$(REPLACESUBSTR$(Path, "\", "/"))
  IF p = LCASE$(REPLACESUBSTR$(ProjectMain, "\", "/")) THEN Result = 1: EXIT FUNCTION
  FOR i = 0 TO Proj.FileCount - 1
    IF p = LCASE$(REPLACESUBSTR$(Proj.FullPath(i), "\", "/")) THEN Result = 1: EXIT FUNCTION
  NEXT
END FUNCTION

' (the problems of Path, a line each: the project's — every file of the
' program checked together, errors first, grouped by file — when Path is
' its main file, else the file's own)
FUNCTION ProblemsText(Path AS STRING) AS STRING
  IF ProjectOpen AND LCASE$(Path) = LCASE$(ProjectMain) AND Lang.Project <> "" THEN
    Result = Lang.ProjectDiagnostics
  ELSE
    Result = Lang.Diagnostics(Path)
  END IF
END FUNCTION

' (how many of each kind, in words: "2 errors, 1 warning, 3 notes")
FUNCTION ProblemCounts(errs AS INTEGER, warns AS INTEGER, notes AS INTEGER) AS STRING
  DIM t AS STRING
  IF errs > 0 THEN t = TRN$("{n} error", "{n} errors", errs) + ", "
  IF warns > 0 THEN t = t + TRN$("{n} warning", "{n} warnings", warns) + ", "
  IF notes > 0 THEN t = t + TRN$("{n} note", "{n} notes", notes) + ", "
  IF t <> "" THEN t = LEFT$(t, LEN(t) - 2)
  Result = t
END FUNCTION

' A file's problems — the main file's: the whole program's —, as the
' language service sorts them (errors first, grouped by file): each file's
' heading with how many of each kind, its problems under it.
SUB FillProblemsOf(Path AS STRING)
  DIM text AS STRING, n AS INTEGER, i AS INTEGER, row AS STRING, sev AS STRING, k AS INTEGER
  DIM f AS STRING, errs AS INTEGER, warns AS INTEGER, notes AS INTEGER
  DIM rowAt AS INTEGER, rowEnd AS INTEGER, at AS INTEGER, nextAt AS INTEGER, rf AS STRING
  text = ProblemsText(Path)
  ' (the same problems as shown: nothing to do — a pause in typing that
  ' changed none of them)
  IF text = ProblemsShownText AND Path = ProblemsShownPath THEN EXIT SUB
  ClearProblems
  ProblemsShownText = text
  ProblemsShownPath = Path
  n = TALLY(text, CHR$(10))
  ' (the rows one after the other, each where the last one ended — FIELD$
  ' for row i went through every row before it, and each file's counts did
  ' it again: thousands of problems took seconds, the hang reports'
  ' ProblemsText)
  rowAt = 1
  FOR i = 1 TO n
    rowEnd = INSTR(rowAt, text, CHR$(10))
    row = MID$(text, rowAt, rowEnd - rowAt)
    rowAt = rowEnd + 1
    ' (a new file: its heading, its counts first — its rows are together)
    IF i = 1 OR FIELD$(row, CHR$(9), 5) <> f THEN
      f = FIELD$(row, CHR$(9), 5)
      errs = 0: warns = 0: notes = 0
      at = rowEnd + 1
      sev = FIELD$(row, CHR$(9), 1)
      DO
        IF sev = "error" THEN
          errs = errs + 1
        ELSEIF sev = "warning" THEN
          warns = warns + 1
        ELSE
          notes = notes + 1
        END IF
        IF at > LEN(text) THEN EXIT DO
        nextAt = INSTR(at, text, CHR$(10))
        IF nextAt = 0 THEN EXIT DO
        rf = MID$(text, at, nextAt - at)
        IF FIELD$(rf, CHR$(9), 5) <> f THEN EXIT DO
        sev = FIELD$(rf, CHR$(9), 1)
        at = nextAt + 1
      LOOP
      ' (its heading: the Project pane's icon, the path from the project's
      ' folder, how many — a pill —, and the counts in words)
      AddFileHeading f, ProblemCounts(errs, warns, notes), errs + warns + notes
    END IF
    sev = FIELD$(row, CHR$(9), 1)
    k = 2
    IF sev = "error" THEN k = 0
    IF sev = "warning" THEN k = 1
    AddProblem k, FIELD$(row, CHR$(9), 4), ProjectRel(f), VAL(FIELD$(row, CHR$(9), 2)), VAL(FIELD$(row, CHR$(9), 3))
  NEXT
END SUB

' The active document's outline, problems and properties again.
SUB AnalyzeActive
  DIM d AS INTEGER
  d = ActiveCode
  ' (not BASIC: no outline, no problems)
  IF d >= 0 THEN
    IF IsBasicFile(DocFile(d)) = 0 THEN d = -1
  END IF
  IF d >= 0 THEN
    Lang.Update DocFile(d), CodeDoc(d).Text
    IF DocHasForm(d) THEN DesignRead d
    ' (not analysed for this text yet — RAI's changes applied, a document
    ' switched to, a file changed outside —: the analysis asked for, off the
    ' UI thread, and the Outline, Problems and Quick Open follow when it's
    ' in (LangAnalyzed); what they showed stays meanwhile. Run and Build ask
    ' for the errors themselves, and wait for them: ProgramErrors)
    IF Lang.Ready(DocFile(d)) = 0 THEN
      Lang.Analyze DocFile(d)
      RefreshProgramLook 0
      InspectActive
      EXIT SUB
    END IF
  END IF
  FillOutline d
  FillProblems d
  ' (Coach: Help With This Error follows the problems — aicoach.inc)
  IF AICoachNow THEN AICoachShow
  ' ($THEME edited by hand in the main file: the designer follows)
  RefreshProgramLook 0
  ' (S-PANELS: the inspector follows the active form's designer)
  InspectActive
  ' (a Markdown file's outline: its headings — markdown.inc)
  IF ActiveCode >= 0 THEN
    IF DocIsMd(ActiveCode) THEN MarkdownOutline ActiveCode
  END IF
END SUB

' ---- opening, saving, closing ---------------------------------------------------------

FUNCTION FreeDoc AS INTEGER
  DIM d AS INTEGER
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) = 0 THEN
      Result = d
      EXIT FUNCTION
    END IF
  NEXT
  IF DocCount < MAXDOCS THEN
    Result = DocCount
    DocCount = DocCount + 1
  ELSE
    Result = -1
  END IF
END FUNCTION

' (the open document of a file, -1 if none)
FUNCTION DocOf(Path AS STRING) AS INTEGER
  DIM d AS INTEGER
  Result = -1
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) AND LCASE$(DocFile(d)) = LCASE$(Path) THEN
      Result = d
      EXIT FUNCTION
    END IF
  NEXT
END FUNCTION

SUB DocTitles(d AS INTEGER)
  ' (S-PANELS: the project tree marks a file with changes not saved; its
  ' tab shows a dot)
  ProjectTree.FileModified DocFile(d), DocDirty(d)
  ' (deleted outside Studio: the tab says so — external.inc)
  IF DocGone(d) THEN
    Dock.PaneTitle(DocPane(d), TR$("{file} (deleted on disk)", "file", FileTitle(DocFile(d))))
  ELSE
    Dock.PaneTitle(DocPane(d), FileTitle(DocFile(d)))
  END IF
  Dock.DocumentModified(DocPane(d), DocDirty(d))
  ' (Studio's window: the dot in macOS' close button, the browser's
  ' question before the page is left)
  UpdateModified
END SUB

' Opens a source file as a document — its tab: its code, and its form in a
' designer when it makes one (workspace.inc). An open one is brought to the
' front as it is.
SUB OpenDocument(Path AS STRING)
  DIM d AS INTEGER
  d = DocOf(Path)
  IF d >= 0 THEN
    Dock.ActiveDocument = DocPane(d)
    EXIT SUB
  END IF
  d = FreeDoc
  IF d < 0 THEN EXIT SUB
  Loading = 1
  CodeDoc(d).Font = MonoFont
  ' (a document closed before left its text here: a file that can't be
  ' read opens empty, never with another file's code under its name)
  CodeDoc(d).Text = ""
  CodeDoc(d).LoadFromFile Path
  CodeDoc(d).Tag = d
  CodeDoc(d).OnChange = CodeChanged
  CodeDoc(d).OnCaretMove = CodeCaretMoved
  CodeDoc(d).OnNavigate = CodeNavigate
  CodeDoc(d).OnReferences = CodeReferences
  CodeDoc(d).OnFileEdits = CodeFileEdits
  ' (its context menu: Go to Definition, Find References … — search.inc)
  CodeMenuFor d
  EditorSettings d
  Loading = 0
  DocFile(d) = Path
  DocKind(d) = "code"
  DocOpen(d) = 1
  DocDirty(d) = 0
  ' (its file as read: changes outside Studio are looked for from here —
  ' external.inc)
  ExtOpened d
  DesignDoc(d).OnSourceEdit = DesignSourceEdit
  DesignDoc(d).OnSourceStep = DesignStep
  DesignDoc(d).OnUndo = DesignUndo
  DesignDoc(d).SharedUndo = 1
  DesignDoc(d).OnChange = DesignChanged
  DesignDoc(d).OnDblClick = DesignDblClick
  DesignRead d
  DocHasForm(d) = Flag(DesignDoc(d).FormName <> "")
  DesignDoc(d).Theme = DesignTheme
  ' (the project's designer settings)
  DesignDoc(d).GridSize = Proj.DesignerGrid
  DesignDoc(d).SnapToGrid = Proj.DesignerSnap
  DesignDoc(d).ShowGuides = Proj.DesignerGuides
  ' (a Markdown file: shown formatted too — markdown.inc)
  DocIsMd(d) = IsMarkdownFile(Path)
  OpenMarkdown d
  AddDocumentTab d
  ActiveDoc = d
  ' (debug.inc: its breakpoints, the paused line, data tips)
  DebugDocOpened d
  ' (versions.inc: its gutter marks against the last version)
  VerApplyBase d
  ' (the project tree lists its components; the others' names are kept
  ' from its designer)
  ProjectFileChanged d
  AnalyzeActive
  ' (editor.inc: the caret's place, encoding and line breaks; the caret in
  ' the code, ready to type, as an editor opens a file)
  EditorFacts d
  ' (a preview from the project tree: the keys stay there)
  IF OpenQuietly = 0 THEN FocusCodeSoon d
  OutputLine TR$("Opened {file}", "file", Path)
  UpdateCommands
END SUB

' Writes a document's file; True when it did.
FUNCTION SaveDocument(d AS INTEGER) AS INTEGER
  IF d < 0 THEN EXIT FUNCTION
  IF DocOpen(d) = 0 THEN EXIT FUNCTION
  ' (the file changed or was deleted outside Studio since it was read:
  ' asked first, never overwritten silently — external.inc)
  IF ExtSaveOk(d) = 0 THEN EXIT FUNCTION
  ' (Versions: what the disk had, kept in the local history first — never
  ' lost, versioned or not: versions.inc)
  VerBeforeSave d
  CodeDoc(d).SaveToFile DocFile(d)
  DocDirty(d) = 0
  ' (Studio's own write: no change from outside)
  ExtSaved d
  ' (saved: no copy of it needed — safety.inc)
  KeepForget d
  DocTitles d
  ProjectFollowsCode
  ' (a copy of what was saved; the versions' badges and counts again)
  VerAfterSave d
  Result = 1
END FUNCTION

' (S1) The project file follows the code: a file saved that now makes a
' form is listed as a form file, one that no longer does as a module — the
' .rrproj written again, the Output says what changed.
SUB ProjectFollowsCode
  DIM changed AS STRING, n AS INTEGER, i AS INTEGER, row AS STRING
  IF ProjectOpen = 0 OR Proj.Kind <> "project" THEN EXIT SUB
  changed = Proj.FollowCode
  IF changed = "" THEN EXIT SUB
  n = TALLY(changed, CHR$(10))
  FOR i = 1 TO n
    row = FIELD$(changed, CHR$(10), i)
    ' (short lines: the Output pane is narrow)
    IF FIELD$(row, CHR$(9), 2) = "form" THEN
      OutputLine TR$("{file} makes a form: now under Forms", "file", FileTitle(FIELD$(row, CHR$(9), 1)))
    ELSEIF FIELD$(row, CHR$(9), 2) = "include" THEN
      OutputLine TR$("{file} makes no form: now under Includes", "file", FileTitle(FIELD$(row, CHR$(9), 1)))
    ELSE
      OutputLine TR$("{file} makes no form: now under Modules", "file", FileTitle(FIELD$(row, CHR$(9), 1)))
    END IF
  NEXT
  SaveProjectFile
END SUB

SUB SaveAll
  DIM ok AS INTEGER
  ok = SaveAllOk
END SUB

' Every document with changes saved; 0 when one wasn't (its file changed
' outside Studio and the question was cancelled): Run, Build and quitting
' stop there.
FUNCTION SaveAllOk AS INTEGER
  DIM d AS INTEGER
  Result = 1
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) AND DocDirty(d) THEN
      IF SaveDocument(d) = 0 THEN Result = 0
    END IF
  NEXT
END FUNCTION

' (the documents of file d go; asked first when it has changes)
SUB CloseDocument(d AS INTEGER)
  IF d < 0 THEN EXIT SUB
  IF DocOpen(d) = 0 THEN EXIT SUB
  ' (its changes: Save / Don't Save / Cancel — workspace.inc)
  IF AskToSave(d) = 0 THEN EXIT SUB
  ' (saved or let go: no copy of it — safety.inc)
  KeepForget d
  ' (no longer watched for changes outside Studio)
  ExtClosed d
  DocOpen(d) = 0
  Dock.ClosePane(DocPane(d))
  Lang.Close DocFile(d)
  IF ActiveDoc = d THEN ActiveDoc = -1
  AnalyzeActive
  UpdateCommands
END SUB

' (POLISH-2) Document d is marked changed while its text differs from what
' was saved: undone back to it, the dot and the title's mark go; redone
' away, they come back (the code editor's Modified compares the text).
SUB DirtyFollows(d AS INTEGER)
  DIM now AS INTEGER
  IF d < 0 OR d >= DocCount THEN EXIT SUB
  IF DocOpen(d) = 0 THEN EXIT SUB
  ' (a file deleted outside Studio stays changed: its text is only here)
  now = Flag(CodeDoc(d).Modified OR DocGone(d))
  IF now <> DocDirty(d) THEN
    DocDirty(d) = now
    DocTitles d
  END IF
END SUB

SUB CodeChanged(Sender AS RCodeEditor)
  DIM d AS INTEGER
  IF Loading THEN EXIT SUB
  d = Sender.Tag
  DirtyFollows d
  ' (its copy kept at the next tick — safety.inc)
  KeepNoted d
  ' (changed: a preview tab is kept — panels.inc)
  PreviewKept d
  CodeTyped d
  ' (the problems and the outline follow once typing stops)
  Analyzer.Enabled = 0
  Analyzer.Enabled = 1
END SUB

' Typing paused: the active document's analysis is asked for — made off the
' UI thread (RLanguageService's Analyze) — and the panes follow when it's in
' (LangAnalyzed): a key typed meanwhile never waits for it.
SUB AnalyzerTick
  DIM d AS INTEGER
  Analyzer.Enabled = 0
  d = ActiveCode
  IF d >= 0 THEN
    IF IsBasicFile(DocFile(d)) THEN
      Lang.Update DocFile(d), CodeDoc(d).Text
      Lang.Analyze DocFile(d)
      ' (S-PANELS: the project tree lists the form's components as the code has them)
      ProjectFileChanged d
      EXIT SUB
    END IF
  END IF
  AnalyzeActive
  ProjectFileChanged ActiveCode
END SUB

' The analysis Analyze asked for is in: the active document's outline,
' problems and the rest, at once — unless it was typed in since (the next
' pause asks again) or another document is active now.
SUB LangAnalyzed(File AS STRING)
  DIM d AS INTEGER
  d = ActiveCode
  IF d < 0 THEN EXIT SUB
  IF LCASE$(REPLACESUBSTR$(DocFile(d), "\", "/")) <> LCASE$(File) THEN EXIT SUB
  ' (typed since: CodeChanged started the Analyzer again — it asks then;
  ' else the text is the one Analyze was given, and the answers are in —
  ' unless another file changed since: AnalyzeActive asks again then)
  IF Analyzer.Enabled THEN EXIT SUB
  IF Lang.Ready(DocFile(d)) = 0 THEN EXIT SUB
  AnalyzeActive
END SUB

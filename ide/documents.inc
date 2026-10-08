' documents.inc: source files as documents — a code editor each, and a
' designer for the form a file creates — and what the side panes show of
' the active one: its outline (RLanguageService), its problems, its form's
' properties.

DIM OutlineLine(400) AS INTEGER
DIM OutlineCount AS INTEGER

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

' (a string literal's text, quotes off)
FUNCTION Unquote(v AS STRING) AS STRING
  IF LEFT$(v, 1) = CHR$(34) AND RIGHT$(v, 1) = CHR$(34) AND LEN(v) >= 2 THEN Result = MID$(v, 2, LEN(v) - 2) ELSE Result = v
END FUNCTION

' (S-PANELS: the stand-in scanner of the form's properties is gone: the
' inspector follows the designer, which reads the code — designer.inc)

' ---- the outline (RLanguageService) ------------------------------------------------

' (an outline kind's icon in SymbolIcons)
FUNCTION OutlineIcon(Kind AS STRING, Detail AS STRING) AS INTEGER
  DIM t AS STRING
  IF Kind = "component" THEN
    t = UCASE$(Detail)
    IF INSTR(t, " ") > 0 THEN t = MID$(t, RINSTR(t, " ") + 1)
    IF t <> "" THEN
      SymbolIcons.AddIcon t
      Result = SymbolIcons.Count - 1
      EXIT FUNCTION
    END IF
  END IF
  SymbolIcons.AddIcon "symbols/" + Kind
  Result = SymbolIcons.Count - 1
END FUNCTION

SUB FillOutline(d AS INTEGER)
  DIM text AS STRING, n AS INTEGER, i AS INTEGER, row AS STRING
  DIM depth AS INTEGER, kind AS STRING, nm AS STRING, detail AS STRING, ln AS INTEGER, img AS INTEGER
  DIM parentNode(20) AS INTEGER
  OutlineTree.Clear
  SymbolIcons.Clear
  OutlineCount = 0
  IF d < 0 THEN EXIT SUB
  text = Lang.Outline(DocFile(d))
  n = TALLY(text, CHR$(10))
  FOR i = 1 TO n
    row = FIELD$(text, CHR$(10), i)
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
END SUB

' ---- problems (RLanguageService's diagnostics: the compiler's messages) ----------

SUB FillProblems(d AS INTEGER)
  DIM text AS STRING, n AS INTEGER, i AS INTEGER, row AS STRING, sev AS STRING, k AS INTEGER, errs AS INTEGER, warns AS INTEGER
  ClearProblems
  IF d < 0 THEN EXIT SUB
  text = Lang.Diagnostics(DocFile(d))
  n = TALLY(text, CHR$(10))
  FOR i = 1 TO n
    row = FIELD$(text, CHR$(10), i)
    sev = FIELD$(row, CHR$(9), 1)
    k = 2
    IF sev = "error" THEN k = 0: errs = errs + 1
    IF sev = "warning" THEN k = 1: warns = warns + 1
    AddProblem k, FIELD$(row, CHR$(9), 4), FIELD$(row, CHR$(9), 5), VAL(FIELD$(row, CHR$(9), 2)), VAL(FIELD$(row, CHR$(9), 3))
  NEXT
END SUB

' The active document's outline, problems and properties again.
SUB AnalyzeActive
  DIM d AS INTEGER
  d = ActiveCode
  IF d >= 0 THEN
    Lang.Update DocFile(d), CodeDoc(d).Text
    IF DocHasForm(d) THEN DesignRead d
  END IF
  FillOutline d
  FillProblems d
  ' (S-PANELS: the inspector follows the active form's designer)
  InspectActive
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
  DIM t AS STRING
  t = FileTitle(DocFile(d))
  IF DocDirty(d) THEN t = t + " *"
  ' (S-PANELS: the project tree marks a file with changes not saved)
  ProjectTree.FileModified DocFile(d), DocDirty(d)
  Dock.PaneTitle("codedoc(" + LTRIM$(STR$(d)) + ")", t)
  IF DocHasForm(d) THEN Dock.PaneTitle("designdoc(" + LTRIM$(STR$(d)) + ")", t + " [Design]")
END SUB

' Opens a source file as documents: its code, and its form in a designer
' when it makes one. An open one is brought to the front.
SUB OpenDocument(Path AS STRING)
  DIM d AS INTEGER, title AS STRING
  d = DocOf(Path)
  IF d >= 0 THEN
    Dock.FocusPane("codedoc(" + LTRIM$(STR$(d)) + ")")
    EXIT SUB
  END IF
  d = FreeDoc
  IF d < 0 THEN EXIT SUB
  Loading = 1
  CodeDoc(d).Font = MonoFont
  CodeDoc(d).LoadFromFile Path
  CodeDoc(d).Tag = d
  CodeDoc(d).OnChange = CodeChanged
  CodeDoc(d).OnCaretMove = CodeCaretMoved
  Loading = 0
  DocFile(d) = Path
  DocKind(d) = "code"
  DocOpen(d) = 1
  DocDirty(d) = 0
  DesignDoc(d).OnSourceEdit = DesignSourceEdit
  DesignDoc(d).OnChange = DesignChanged
  DesignDoc(d).OnDblClick = DesignDblClick
  DesignRead d
  DocHasForm(d) = Flag(DesignDoc(d).FormName <> "")
  title = FileTitle(Path)
  IF DocHasForm(d) THEN Dock.AddPane(DesignDoc(d), title + " [Design]", "documents", "form")
  Dock.AddPane(CodeDoc(d), title, "documents", "code")
  ActiveDoc = d
  ' (debug.inc: its breakpoints, the paused line, data tips)
  DebugDocOpened d
  AnalyzeActive
  ' (editor.inc: the caret's place, encoding and line breaks; the caret in
  ' the code, ready to type, as an editor opens a file)
  EditorFacts d
  FocusCodeSoon d
  OutputLine "Opened " + Path
  UpdateCommands
END SUB

' Writes a document's file; True when it did.
FUNCTION SaveDocument(d AS INTEGER) AS INTEGER
  IF d < 0 THEN EXIT FUNCTION
  IF DocOpen(d) = 0 THEN EXIT FUNCTION
  CodeDoc(d).SaveToFile DocFile(d)
  DocDirty(d) = 0
  DocTitles d
  Result = 1
END FUNCTION

SUB SaveAll
  DIM d AS INTEGER
  FOR d = 0 TO DocCount - 1
    IF DocOpen(d) AND DocDirty(d) THEN SaveDocument d
  NEXT
END SUB

' (the documents of file d go; asked first when it has changes)
SUB CloseDocument(d AS INTEGER)
  IF d < 0 THEN EXIT SUB
  IF DocOpen(d) = 0 THEN EXIT SUB
  DocOpen(d) = 0
  Dock.ClosePane("codedoc(" + LTRIM$(STR$(d)) + ")")
  IF DocHasForm(d) THEN Dock.ClosePane("designdoc(" + LTRIM$(STR$(d)) + ")")
  Lang.Close DocFile(d)
  IF ActiveDoc = d THEN ActiveDoc = -1
  AnalyzeActive
  UpdateCommands
END SUB

SUB CodeChanged(Sender AS RCODEEDITOR)
  DIM d AS INTEGER
  IF Loading THEN EXIT SUB
  d = Sender.Tag
  IF DocDirty(d) = 0 THEN
    DocDirty(d) = 1
    DocTitles d
  END IF
  ' (the problems and the outline follow once typing stops)
  Analyzer.Enabled = 0
  Analyzer.Enabled = 1
END SUB

SUB AnalyzerTick
  Analyzer.Enabled = 0
  AnalyzeActive
  ' (S-PANELS: the project tree lists the form's components as the code has them)
  ProjectFileChanged ActiveCode
END SUB

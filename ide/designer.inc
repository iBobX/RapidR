' designer.inc: RapidR Studio's form designer (docs/ide-plan.md I4) — each
' source file's [Design] document is an RDESIGNSURFACE reading the file's
' text (Source): its form drawn as the running program draws it, every
' change written back into the code as the smallest edits (OnSourceEdit),
' one undo history of exact bytes per file.
'
' The surface's entry points for the other panes (docs/ide-plan.md, "I4 /
' S-DESIGN results"):
'   DesignDoc(d).PlaceType = "QBUTTON"         the toolbox's click: the next
'                                              click or drag on the form places one
'   DesignDoc(d).DragComponent "QBUTTON"       the toolbox's mouse-down: dropped
'                                              where the mouse is let go
'   DesignDoc(d).AddComponent "QBUTTON", -1, -1   Enter on a toolbox item
'   DesignDoc(d).CreateHandler(Name, Event)    a handler made or found
'   DesignDoc(d).SelectName Name, .SelIndex, .GetName(i), OnSelect, OnChange

' ---- one undo history per file (docs/ide-plan.md, "The shared undo") ------
' The designer's changes (OnSourceStep, then its OnSourceEdits) and what is
' typed in the code are steps of ONE history, undone in the order they
' were made, from either document. The contract puts that history in the
' code editor (RCODEEDITOR.ApplyPatches / Undo / Redo, S-EDITOR's); until
' it has them, Studio keeps it here: each step a list of patches (where,
' the text taken out, the text put in), typing noted as one step when the
' analyzer's pause comes (HistNoteCode). The swap: DesignStep +
' DesignSourceEdit -> ApplyPatches at OnChange, HistStep -> CodeDoc(d).Undo
' / .Redo, and this section goes.
CONST HISTMAX = 200
DIM HUndo(MAXDOCS, HISTMAX) AS STRING
DIM HRedo(MAXDOCS, HISTMAX) AS STRING
DIM HUndoN(MAXDOCS) AS INTEGER
DIM HRedoN(MAXDOCS) AS INTEGER
' (the code as the history last saw it: what typing changed is the rest)
DIM HShadow(MAXDOCS) AS STRING
' (OnSourceStep heard: the next edit begins a step, 1, or adds to the last, 2)
DIM HPend(MAXDOCS) AS INTEGER

' Document d's history starts over (its file opened).
SUB HistReset(d AS INTEGER)
  HUndoN(d) = 0
  HRedoN(d) = 0
  HPend(d) = 0
  HShadow(d) = CodeDoc(d).Text
END SUB

' (a step on top of document d's undo stack; the oldest goes when full)
SUB HistPush(d AS INTEGER, s AS STRING)
  DIM k AS INTEGER
  IF HUndoN(d) >= HISTMAX THEN
    FOR k = 1 TO HISTMAX - 1
      HUndo(d, k - 1) = HUndo(d, k)
    NEXT
    HUndoN(d) = HISTMAX - 1
  END IF
  HUndo(d, HUndoN(d)) = s
  HUndoN(d) = HUndoN(d) + 1
END SUB

' (the longest common start and end of a and b: the one patch between them)
FUNCTION HistDiff(a AS STRING, b AS STRING) AS STRING
  DIM lo AS INTEGER, hi AS INTEGER, md AS INTEGER, p AS INTEGER, q AS INTEGER, m AS INTEGER
  m = LEN(a)
  IF LEN(b) < m THEN m = LEN(b)
  lo = 0: hi = m
  WHILE lo < hi
    md = (lo + hi + 1) \ 2
    IF LEFT$(a, md) = LEFT$(b, md) THEN lo = md ELSE hi = md - 1
  WEND
  p = lo
  lo = 0: hi = m - p
  WHILE lo < hi
    md = (lo + hi + 1) \ 2
    IF RIGHT$(a, md) = RIGHT$(b, md) THEN lo = md ELSE hi = md - 1
  WEND
  q = lo
  Result = LTRIM$(STR$(p)) + CHR$(1) + MID$(a, p + 1, LEN(a) - p - q) + CHR$(1) + MID$(b, p + 1, LEN(b) - p - q)
END FUNCTION

' What was typed in document d's code since the history last looked: one
' step (the designer's next change, or an undo, comes after it).
SUB HistNoteCode(d AS INTEGER)
  DIM t AS STRING
  IF d < 0 THEN EXIT SUB
  IF DocOpen(d) = 0 THEN EXIT SUB
  t = CodeDoc(d).Text
  IF t = HShadow(d) THEN EXIT SUB
  HistPush d, HistDiff(HShadow(d), t)
  HRedoN(d) = 0
  HShadow(d) = t
END SUB

' (Studio's edit of document d's code: offset a, n characters replaced by
' Text — the caret kept where it was in the text around it)
SUB HistReplace(d AS INTEGER, a AS INTEGER, n AS INTEGER, Text AS STRING)
  DIM caret AS INTEGER, b AS INTEGER
  b = a + n
  caret = CodeDoc(d).SelStart
  CodeDoc(d).SelStart = a
  CodeDoc(d).SelLength = n
  CodeDoc(d).SelText = Text
  IF caret >= b THEN
    caret = caret + LEN(Text) - n
  ELSEIF caret > a THEN
    caret = a
  END IF
  CodeDoc(d).SelStart = caret
  CodeDoc(d).SelLength = 0
END SUB

' Undo (Back) or Redo in document d's history: the step's patches taken
' back (last first) or made again; the designer reads the code again.
' True when there was a step.
FUNCTION HistStep(d AS INTEGER, Back AS INTEGER) AS INTEGER
  DIM s AS STRING, pt AS STRING, i AS INTEGER, n AS INTEGER, k AS INTEGER, a AS INTEGER, was AS STRING, now AS STRING
  IF d < 0 THEN EXIT FUNCTION
  IF DocOpen(d) = 0 THEN EXIT FUNCTION
  HistNoteCode d
  IF Back THEN
    IF HUndoN(d) = 0 THEN EXIT FUNCTION
    HUndoN(d) = HUndoN(d) - 1
    s = HUndo(d, HUndoN(d))
    IF HRedoN(d) < HISTMAX THEN
      HRedo(d, HRedoN(d)) = s
      HRedoN(d) = HRedoN(d) + 1
    END IF
  ELSE
    IF HRedoN(d) = 0 THEN EXIT FUNCTION
    HRedoN(d) = HRedoN(d) - 1
    s = HRedo(d, HRedoN(d))
    HistPush d, s
  END IF
  IF s <> "" THEN
    n = TALLY(s, CHR$(2)) + 1
    FOR k = 1 TO n
      IF Back THEN i = n - k + 1 ELSE i = k
      pt = FIELD$(s, CHR$(2), i)
      a = VAL(FIELD$(pt, CHR$(1), 1))
      was = FIELD$(pt, CHR$(1), 2)
      now = FIELD$(pt, CHR$(1), 3)
      IF Back THEN HistReplace d, a, LEN(now), was ELSE HistReplace d, a, LEN(was), now
    NEXT
  END IF
  HShadow(d) = CodeDoc(d).Text
  IF DocDirty(d) = 0 THEN
    DocDirty(d) = 1
    DocTitles d
  END IF
  IF DocHasForm(d) THEN DesignRead d
  Analyzer.Enabled = 0
  Analyzer.Enabled = 1
  UpdateCommands
  Result = 1
END FUNCTION

' The designer's change begins (OnSourceStep): a new step of the file's
' history, or — Continues — more of the last one.
SUB DesignStep(Continues AS INTEGER, Sender AS RDESIGNSURFACE)
  DIM d AS INTEGER
  d = Sender.Tag
  IF d < 0 OR d >= DocCount THEN EXIT SUB
  HistNoteCode d
  ' (the step begins with its first edit: a change that writes nothing
  ' leaves no empty step)
  IF Continues THEN HPend(d) = 2 ELSE HPend(d) = 1
END SUB

' Ctrl+Z / Ctrl+Y on the designer (OnUndo, SharedUndo): the file's history.
SUB DesignUndo(Redo AS INTEGER, Sender AS RDESIGNSURFACE)
  DIM ok AS INTEGER
  IF Redo THEN ok = HistStep(Sender.Tag, 0) ELSE ok = HistStep(Sender.Tag, 1)
END SUB

' (the active document's designer, -1: none)
FUNCTION ActiveDesign AS INTEGER
  Result = -1
  IF ActiveDoc >= 0 THEN
    IF DocOpen(ActiveDoc) AND DocHasForm(ActiveDoc) THEN Result = ActiveDoc
  END IF
END FUNCTION

' Document d's designer reads its code (when it opens, when the code was
' edited: the same text again changes nothing).
SUB DesignRead(d AS INTEGER)
  DesignDoc(d).Tag = d
  DesignDoc(d).SourceFile = DocFile(d)
  DesignDoc(d).Source = CodeDoc(d).Text
END SUB

' (character offset of line l, column c in document d's code: the code
' editor's SelStart)
FUNCTION CodeOffset(d AS INTEGER, l AS INTEGER, c AS INTEGER) AS INTEGER
  DIM k AS INTEGER, o AS INTEGER
  FOR k = 0 TO l - 1
    o = o + LEN(CodeDoc(d).Line(k)) + 1
  NEXT
  Result = o + c
END FUNCTION

' The designer changed the code: the edit applied to the code editor (the
' caret kept where it was in the text around it).
SUB DesignSourceEdit(StartLine AS INTEGER, StartCol AS INTEGER, EndLine AS INTEGER, EndCol AS INTEGER, Text AS STRING, Sender AS RDESIGNSURFACE)
  DIM d AS INTEGER, a AS INTEGER, b AS INTEGER, k AS INTEGER, was AS STRING
  d = Sender.Tag
  IF d < 0 OR d >= DocCount THEN EXIT SUB
  IF DocOpen(d) = 0 THEN EXIT SUB
  a = CodeOffset(d, StartLine, StartCol)
  b = CodeOffset(d, EndLine, EndCol)
  ' (the patch goes into the step OnSourceStep began)
  was = MID$(HShadow(d), a + 1, b - a)
  IF HPend(d) = 1 OR HUndoN(d) = 0 THEN HistPush d, ""
  IF HPend(d) THEN HRedoN(d) = 0
  HPend(d) = 0
  k = HUndoN(d) - 1
  IF HUndo(d, k) <> "" THEN HUndo(d, k) = HUndo(d, k) + CHR$(2)
  HUndo(d, k) = HUndo(d, k) + LTRIM$(STR$(a)) + CHR$(1) + was + CHR$(1) + Text
  HistReplace d, a, b - a, Text
  HShadow(d) = LEFT$(HShadow(d), a) + Text + MID$(HShadow(d), b + 1)
  ' (the file has changes; the outline and problems follow)
  IF DocDirty(d) = 0 THEN
    DocDirty(d) = 1
    DocTitles d
  END IF
  Analyzer.Enabled = 0
  Analyzer.Enabled = 1
END SUB

SUB DesignChanged(Sender AS RDESIGNSURFACE)
  UpdateCommands
END SUB

' Document d's designer shown (and focused) — for a file that makes no
' form too: its empty state offers "Add a Form".
SUB ShowDesigner(d AS INTEGER)
  IF d < 0 THEN EXIT SUB
  IF DocOpen(d) = 0 THEN EXIT SUB
  IF DocHasForm(d) = 0 THEN
    DocHasForm(d) = 1
    Dock.AddPane(DesignDoc(d), FileTitle(DocFile(d)) + " [Design]", "documents", "form")
  END IF
  Dock.FocusPane("designdoc(" + LTRIM$(STR$(d)) + ")")
END SUB

' A component double-clicked (or Enter on it): its default event's handler
' — made and wired when there's none — in the code editor.
SUB DesignDblClick(Index AS INTEGER, Sender AS RDESIGNSURFACE)
  DIM d AS INTEGER, nm AS STRING, s AS STRING
  d = Sender.Tag
  nm = Sender.GetName(Index)
  s = Sender.CreateHandler(nm, "")
  IF s = "" THEN
    OutputLine "Designer: " + Sender.StatusText
    EXIT SUB
  END IF
  Dock.FocusPane("codedoc(" + LTRIM$(STR$(d)) + ")")
  IF Sender.HandlerLine >= 0 THEN CodeDoc(d).GotoLine Sender.HandlerLine
END SUB

' The designer's commands (Format, Edit on a designer; View's switches).
' True when Id was one.
FUNCTION DesignCommand(Id AS STRING) AS INTEGER
  DIM d AS INTEGER, how AS STRING, k AS INTEGER
  d = ActiveDesign
  Result = 1
  SELECT CASE Id
    CASE "view.designer"
      ShowDesigner ActiveDoc
      EXIT FUNCTION
    CASE "designer.addForm"
      ' (Project > Add a Form to This File: a file without one gets one)
      IF ActiveDoc >= 0 THEN
        ShowDesigner ActiveDoc
        DesignDoc(ActiveDoc).AddForm ""
      END IF
      EXIT FUNCTION
    CASE "view.code"
      IF ActiveDoc >= 0 THEN Dock.FocusPane("codedoc(" + LTRIM$(STR$(ActiveDoc)) + ")")
      EXIT FUNCTION
  END SELECT
  IF LEFT$(Id, 14) = "designer.place" THEN
    ' (designer.place.QBUTTON: the placing tool, as the toolbox's click)
    IF d >= 0 THEN DesignDoc(d).PlaceType = MID$(Id, 16)
    EXIT FUNCTION
  END IF
  IF LEFT$(Id, 12) = "designer.add" THEN
    ' (designer.add.QBUTTON: added at a free spot, as Enter on the toolbox)
    IF d >= 0 THEN DesignDoc(d).AddComponent MID$(Id, 14), -1, -1
    EXIT FUNCTION
  END IF
  ' (Edit > Undo / Redo: the file's one history, from its code or its designer)
  IF Id = "edit.undo" OR Id = "edit.redo" THEN
    IF ActiveDoc >= 0 THEN
      IF Id = "edit.undo" THEN k = HistStep(ActiveDoc, 1) ELSE k = HistStep(ActiveDoc, 0)
    END IF
    EXIT FUNCTION
  END IF
  IF d < 0 OR DocKind(d) <> "design" THEN
    Result = 0
    EXIT FUNCTION
  END IF
  SELECT CASE Id
    CASE "designer.alignLeft": how = "left"
    CASE "designer.alignRight": how = "right"
    CASE "designer.alignTop": how = "top"
    CASE "designer.alignBottom": how = "bottom"
    CASE "designer.alignCenters": how = "center"
    CASE "designer.alignMiddles": how = "middle"
    CASE "designer.sameWidth": how = "samewidth"
    CASE "designer.sameHeight": how = "sameheight"
    CASE "designer.sameSize": how = "samesize"
    CASE "designer.centerHorizontally": how = "centerh"
    CASE "designer.centerVertically": how = "centerv"
    CASE "designer.spaceHorizontally": how = "spaceh"
    CASE "designer.spaceVertically": how = "spacev"
    CASE "designer.bringToFront": how = "front"
    CASE "designer.sendToBack": how = "back"
    CASE "designer.zoomIn": DesignDoc(d).ZoomStep 1: EXIT FUNCTION
    CASE "designer.zoomOut": DesignDoc(d).ZoomStep -1: EXIT FUNCTION
    CASE "designer.zoomReset": DesignDoc(d).ZoomStep 0: EXIT FUNCTION
    CASE "designer.zoomToFit": DesignDoc(d).ZoomToFit: EXIT FUNCTION
    CASE "designer.editCaption": DesignDoc(d).EditCaption: EXIT FUNCTION
    CASE "designer.menuEditor": DesignDoc(d).EditMenu: EXIT FUNCTION
    CASE "designer.tabOrder"
      IF DesignDoc(d).TabOrderMode THEN DesignDoc(d).TabOrderMode = 0 ELSE DesignDoc(d).TabOrderMode = 1
      EXIT FUNCTION
    CASE "edit.cut": Clipboard.Text = DesignDoc(d).CutSelection: EXIT FUNCTION
    CASE "edit.copy": Clipboard.Text = DesignDoc(d).CopySelection: EXIT FUNCTION
    CASE "edit.paste": DesignDoc(d).Paste: EXIT FUNCTION
    CASE "edit.selectAll": DesignDoc(d).SelectAll: EXIT FUNCTION
    CASE "edit.delete": DesignDoc(d).DeleteSelection: EXIT FUNCTION
    CASE "edit.duplicate": DesignDoc(d).Duplicate: EXIT FUNCTION
    CASE ELSE
      Result = 0
      EXIT FUNCTION
  END SELECT
  DesignDoc(d).Arrange how
END FUNCTION

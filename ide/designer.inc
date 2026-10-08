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
' The designer's changes and what is typed in the code are steps of ONE
' history, the code editor's (RCODEEDITOR's ApplyPatches / Undo / Redo):
' each designer change is one step (OnSourceStep says whether it joins the
' step before), and Undo / Redo on the designer (SharedUndo: OnUndo) undo
' there, the designer reading the code again.
DIM DesignCont(MAXDOCS) AS INTEGER

' A designer change begins (OnSourceStep): its patches follow, then OnChange.
SUB DesignStep(Continues AS INTEGER, Sender AS RDESIGNSURFACE)
  DIM d AS INTEGER
  d = Sender.Tag
  IF d < 0 OR d >= DocCount THEN EXIT SUB
  DesignCont(d) = Flag(Continues)
END SUB

' Undo (Redo False) or Redo in document d's one history: the code
' editor's; the designer follows.
SUB FileUndo(d AS INTEGER, Redo AS INTEGER)
  IF d < 0 OR d >= DocCount THEN EXIT SUB
  IF DocOpen(d) = 0 THEN EXIT SUB
  IF Redo THEN CodeDoc(d).Redo ELSE CodeDoc(d).Undo
  IF DocHasForm(d) THEN DesignRead d
  ' (the inspector shows the values undone)
  InspectActive
  UpdateCommands
END SUB

' Ctrl+Z / Ctrl+Y on the designer (OnUndo, SharedUndo): the file's history.
SUB DesignUndo(Redo AS INTEGER, Sender AS RDESIGNSURFACE)
  FileUndo Sender.Tag, Redo
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

' The designer changed the code: each OnSourceEdit patch (lines from 0,
' columns in characters, in the text the ones before left) is kept until
' the change's OnChange, then the code editor applies them all as one undo
' step (RCODEEDITOR's ApplyPatches; the carets keep their places in the
' text, and its OnChange marks the file changed and wakes the analyzer).
DIM DesignPatches(MAXDOCS) AS STRING

' (a patch's text as ApplyPatches reads it: \, line breaks and tabs escaped)
FUNCTION PatchText(Text AS STRING) AS STRING
  Result = REPLACESUBSTR$(Text, "\", "\\")
  Result = REPLACESUBSTR$(Result, CHR$(10), "\n")
  Result = REPLACESUBSTR$(Result, CHR$(13), "\r")
  Result = REPLACESUBSTR$(Result, CHR$(9), "\t")
END FUNCTION

SUB DesignSourceEdit(StartLine AS INTEGER, StartCol AS INTEGER, EndLine AS INTEGER, EndCol AS INTEGER, Text AS STRING, Sender AS RDESIGNSURFACE)
  DIM d AS INTEGER
  d = Sender.Tag
  IF d < 0 OR d >= DocCount THEN EXIT SUB
  DesignPatches(d) = DesignPatches(d) + STR$(StartLine) + CHR$(9) + STR$(StartCol) + CHR$(9) + STR$(EndLine) + CHR$(9) + STR$(EndCol) + CHR$(9) + PatchText(Text) + CHR$(10)
END SUB

SUB DesignChanged(Sender AS RDESIGNSURFACE)
  DIM d AS INTEGER
  d = Sender.Tag
  IF d >= 0 AND d < DocCount THEN
    IF DesignPatches(d) <> "" THEN
      IF DocOpen(d) THEN CodeDoc(d).ApplyPatches DesignPatches(d), DesignCont(d)
      DesignCont(d) = 0
      DesignPatches(d) = ""
    END IF
  END IF
  UpdateCommands
END SUB

' Document d's designer shown (and focused) — for a file that makes no
' form too: its tab gets the Design | Code switch, and the designer's empty
' state offers "Add a Form".
SUB ShowDesigner(d AS INTEGER)
  IF d < 0 THEN EXIT SUB
  IF DocOpen(d) = 0 THEN EXIT SUB
  IF DocHasForm(d) = 0 THEN
    DocHasForm(d) = 1
    Dock.AddView(DocPane(d), DesignDoc(d), "Design")
    Dock.AddView(DocPane(d), CodeDoc(d), "Code")
    DesignDoc(d).OnSelect = DesignPicked
  END IF
  IF DocViewNow(d) = "Code" OR DocViewNow(d) = "" THEN Dock.DocumentView(DocPane(d), "Design")
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
  IF Sender.HandlerLine >= 0 THEN CaretAtHandler d, Sender.HandlerLine
END SUB

' The designer's commands (Format, Edit on a designer; View's switches).
' True when Id was one.
FUNCTION DesignCommand(Id AS STRING) AS INTEGER
  DIM d AS INTEGER, how AS STRING
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
    ' (the file's one history, the code editor's: editor.inc's on the code)
    CASE "edit.undo": FileUndo d, 0: EXIT FUNCTION
    CASE "edit.redo": FileUndo d, 1: EXIT FUNCTION
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

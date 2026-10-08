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
  DIM d AS INTEGER, a AS INTEGER, b AS INTEGER, caret AS INTEGER
  d = Sender.Tag
  IF d < 0 OR d >= DocCount THEN EXIT SUB
  IF DocOpen(d) = 0 THEN EXIT SUB
  a = CodeOffset(d, StartLine, StartCol)
  b = CodeOffset(d, EndLine, EndCol)
  caret = CodeDoc(d).SelStart
  CodeDoc(d).SelStart = a
  CodeDoc(d).SelLength = b - a
  CodeDoc(d).SelText = Text
  IF caret >= b THEN
    caret = caret + LEN(Text) - (b - a)
  ELSEIF caret > a THEN
    caret = a
  END IF
  CodeDoc(d).SelStart = caret
  CodeDoc(d).SelLength = 0
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
  CodeDoc(d).GotoLine Sender.HandlerLine
END SUB

' The designer's commands (Format, Edit on a designer; View's switches).
' True when Id was one.
FUNCTION DesignCommand(Id AS STRING) AS INTEGER
  DIM d AS INTEGER, how AS STRING
  d = ActiveDesign
  Result = 1
  SELECT CASE Id
    CASE "view.designer"
      IF d >= 0 THEN Dock.FocusPane("designdoc(" + LTRIM$(STR$(d)) + ")")
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
    CASE "edit.undo": DesignDoc(d).Undo: EXIT FUNCTION
    CASE "edit.redo": DesignDoc(d).Redo: EXIT FUNCTION
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

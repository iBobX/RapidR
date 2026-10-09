' RCODEEDITOR's debugger markers (RapidR Studio's debugger, S-DEBUG's
' names): AddMarker's breakpoint kinds drawn as the gutter's dot (a dot, a
' dot with a bar, a diamond, a ring); "current" (an arrow, the line
' tinted), "frame" (a grey arrow, a fainter tint) and "exception" (an arrow
' and the line in the error colour) drawn over the dot; a marker's note as
' a rounded label after the line's end (the newest one on a line); WordAt's
' dotted names; DebugHover. ApplyPatches (the designer's changes, one undo
' step) and OnChange after it, Undo and Redo.
DECLARE SUB Go
DECLARE SUB Changed
CREATE Form AS QFORM
  Caption = "markers"
  Width = 560: Height = 330
  CREATE Ed AS RCODEEDITOR
    Left = 8: Top = 8: Width = 520: Height = 220
    OnChange = Changed
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 236: Width = 520
  END CREATE
  CREATE Btn AS QBUTTON
    Top = 262
    OnClick = Go
  END CREATE
END CREATE
DIM Changes AS INTEGER
L$ = CHR$(10)
Ed.Text = "SUB Divide(a AS INTEGER)" + L$ + "  PRINT 10 / a" + L$ + "END SUB" + L$ + "DIM x AS INTEGER" + L$ + "x = 3" + L$ + "Form.Caption = STR$(x)" + L$ + "Divide 0" + L$ + "PRINT x" + L$ + "PRINT x + 1" + L$ + "PRINT x + 2"
Ed.AddMarker 1, "breakpoint"
Ed.AddMarker 4, "breakpoint.conditional"
Ed.AddMarker 5, "breakpoint.log"
Ed.AddMarker 6, "breakpoint.disabled"
Ed.AddMarker 2, "breakpoint"
Ed.AddMarker 2, "exception", "Division by zero"
Ed.AddMarker 7, "frame"
Ed.AddMarker 9, "breakpoint", "hit 3"
Ed.AddMarker 9, "current"
Ed.DebugHover = 1
Lbl.Caption = Ed.WordAt(6, 8) + "|" + Ed.WordAt(6, 2) + "|" + Ed.GetMarkers("breakpoint") + "|" + STR$(Ed.DebugHover)
Form.ShowModal

SUB Changed
  Changes = Changes + 1
END SUB

SUB Go
  T$ = CHR$(9)
  A = Ed.ApplyPatches("3" + T$ + "4" + T$ + "3" + T$ + "5" + T$ + "y" + L$ + "4" + T$ + "0" + T$ + "4" + T$ + "1" + T$ + "y", 0)
  B$ = Ed.Line(3) + "/" + Ed.Line(4)
  Ed.Undo
  Lbl.Caption = Lbl.Caption + "|" + STR$(A) + "|" + B$ + "|" + Ed.Line(3) + "|" + STR$(Changes)
END SUB

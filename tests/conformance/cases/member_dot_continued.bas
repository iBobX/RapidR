' A `_` right after a member's dot joins the next line's name to it, its
' indentation dropped: RC.EXE (RapidQ 2006, in the Windows VM:
' docs/rapidq-ground-truth.md) ran this and printed "hi" (RapidQ's own
' forms/MultiCaptiveChildWnds.bas writes `REDITPOP(i)._` then `POPUP(x, y)`).
' A dot that ends the line without it is an error (member_dot_alone).
DIM Form AS QFORM
Form._
    Caption = "hi"
PRINT Form.Caption

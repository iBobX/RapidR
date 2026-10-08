' QFORMMDI's child window sized by the mouse as Windows' MDI children are:
' its right edge, its bottom-right corner, its top-left corner dragged
' (the test hooks' mouse on the child's frame), each OnChildResize heard;
' then a drag past Windows' least size stops at it.
$INCLUDE "RAPIDQ.INC"
DIM ed AS QRICHEDIT
DIM resizes AS INTEGER

SUB Resized (h AS LONG, i AS INTEGER, t AS STRING)
  resizes = resizes + 1
END SUB

SUB AddIt
  Form.AddChild(ed.Handle, "One", 0, 40, 30, 300, 200, 0)
END SUB

SUB Report
  lbl.Caption = STR$(Form.ChildLeft) + "," + STR$(Form.ChildTop) + "," + STR$(Form.ChildWidth) + "," + STR$(Form.ChildHeight) + "|r" + STR$(resizes)
END SUB

CREATE Form AS QFORMMDI
  Caption = "MDI edges"
  Width = 620: Height = 460
  OnChildResize = Resized
  CREATE bAdd AS QBUTTON
    Caption = "Add": Left = 0: Top = 380: OnClick = AddIt
  END CREATE
  CREATE bReport AS QBUTTON
    Caption = "Report": Left = 80: Top = 380: OnClick = Report
  END CREATE
  CREATE lbl AS QLABEL
    Left = 160: Top = 384: Width = 400
  END CREATE
END CREATE
ed.Parent = Form
ed.Visible = False
Form.ShowModal

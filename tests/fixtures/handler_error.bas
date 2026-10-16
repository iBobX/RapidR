' A run-time error in an event handler, as RapidQ's (RC.EXE, DEBUG-2): the
' handler stops at the error — and the handler that fired it (Outer's
' Box.Text = … runs Changed, which fails) —, no AFTER is written, and the
' form goes on answering (Fine still works). tests/native_gui_events.mjs.
DECLARE SUB Oops
DECLARE SUB Fine
DECLARE SUB Outer
DECLARE SUB Changed
DIM zero AS INTEGER, x AS INTEGER, trail AS STRING
CREATE Form AS QFORM
  Caption = "Err": Width = 300: Height = 200
  CREATE OopsBtn AS QBUTTON
    Left = 10: Top = 10: Caption = "Oops"
    OnClick = Oops
  END CREATE
  CREATE FineBtn AS QBUTTON
    Left = 10: Top = 50: Caption = "Fine"
    OnClick = Fine
  END CREATE
  CREATE OuterBtn AS QBUTTON
    Left = 10: Top = 90: Caption = "Outer"
    OnClick = Outer
  END CREATE
  CREATE Box AS QEDIT
    Left = 120: Top = 10
    OnChange = Changed
  END CREATE
  CREATE Steps AS QLABEL
    Left = 10: Top = 130: Width = 280
  END CREATE
END CREATE
SUB Oops
  trail = trail + " oops"
  x = 1 \ zero
  trail = trail + " AFTER"
END SUB
SUB Fine
  trail = trail + " fine"
  Steps.Caption = trail
END SUB
SUB Changed
  trail = trail + " changed"
  x = 1 \ zero
  trail = trail + " AFTER-CHANGED"
END SUB
SUB Outer
  trail = trail + " outer"
  Box.Text = "x" + trail
  trail = trail + " AFTER-OUTER"
END SUB
Form.ShowModal

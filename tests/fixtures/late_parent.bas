' Components given a parent after the form is shown (in an event handler)
' get their widget then, as in RapidQ: a button, and a panel holding a label.
$INCLUDE "RAPIDQ.INC"
DIM Late AS QBUTTON
DIM Box AS QPANEL
DIM Inner AS QLABEL

SUB Clicked
  lbl.Caption = "late clicked"
END SUB

SUB MakeIt
  Late.Parent = Form
  Late.Left = 10: Late.Top = 40: Late.Width = 90
  Late.Caption = "late"
  Late.OnClick = Clicked
  Box.Parent = Form
  Box.Left = 120: Box.Top = 40: Box.Width = 120: Box.Height = 40
  Inner.Parent = Box
  Inner.Left = 5: Inner.Top = 5
  Inner.Caption = "inside"
END SUB

CREATE Form AS QFORM
  Caption = "Late parent"
  Width = 300: Height = 160
  CREATE Btn AS QBUTTON
    Caption = "Make": Left = 10: Top = 5
    OnClick = MakeIt
  END CREATE
  CREATE lbl AS QLABEL
    Left = 10: Top = 100: Width = 200
    Caption = "-"
  END CREATE
END CREATE
Form.ShowModal

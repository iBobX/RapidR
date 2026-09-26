' An array of buttons sharing one OnClick handler; Sender says which one.
$INCLUDE "RAPIDQ.INC"
CREATE Form AS QFORM
  Caption = "Button array"
  Width = 360
  Height = 120
END CREATE
DIM Btn(1 TO 3) AS QBUTTON
DIM i AS INTEGER
FOR i = 1 TO 3
  Btn(i).Parent = Form
  Btn(i).Left = (i - 1) * 110 + 10
  Btn(i).Top = 10
  Btn(i).Caption = "Button" + STR$(i)
  Btn(i).OnClick = Clicked
NEXT i
SUB Clicked (Sender AS QBUTTON)
  Sender.Caption = "Hit " + Sender.Caption
END SUB
Form.ShowModal

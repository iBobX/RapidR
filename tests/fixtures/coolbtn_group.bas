' QCOOLBTN groups (manual: GroupIndex, Down, AllowAllUp): one button of a
' group is down; pressing another releases it; the one down stays down
' unless AllowAllUp; Down = True from the program releases the others;
' GroupIndex 0 never stays down. OnClick reads the new Down.
$INCLUDE "RAPIDQ.INC"
DIM log AS STRING

SUB Clicked (Sender AS QCOOLBTN)
  log = log + Sender.Caption + STR$(Sender.Down <> 0) + " "
  Show
END SUB

CREATE Form AS QFORM
  Caption = "Cool buttons"
  Width = 420
  Height = 140
  CREATE A AS QCOOLBTN
    Caption = "A": Left = 10: Top = 10: GroupIndex = 1: Down = True
    OnClick = Clicked
  END CREATE
  CREATE B AS QCOOLBTN
    Caption = "B": Left = 90: Top = 10: GroupIndex = 1
    OnClick = Clicked
  END CREATE
  CREATE C AS QCOOLBTN
    Caption = "C": Left = 170: Top = 10: GroupIndex = 1
    OnClick = Clicked
  END CREATE
  CREATE D AS QCOOLBTN
    Caption = "D": Left = 250: Top = 10: GroupIndex = 0
    OnClick = Clicked
  END CREATE
  CREATE E AS QCOOLBTN
    Caption = "E": Left = 330: Top = 10: GroupIndex = 2: AllowAllUp = True
    OnClick = Clicked
  END CREATE
  CREATE Setter AS QBUTTON
    Caption = "C down": Left = 10: Top = 50
    OnClick = SetC
  END CREATE
  CREATE lbl AS QLABEL
    Left = 10: Top = 90: Width = 400
  END CREATE
END CREATE

SUB Show
  lbl.Caption = log + "|" + STR$(A.Down <> 0) + STR$(B.Down <> 0) + STR$(C.Down <> 0) + STR$(D.Down <> 0) + STR$(E.Down <> 0)
END SUB

SUB SetC
  C.Down = True
  log = log + "setC "
  Show
END SUB

Form.ShowModal

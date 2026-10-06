' Push buttons: the focus on an edit, so the Default button shows its
' frame; a normal, a disabled, a held-down and a tall one; cool buttons
' (raised, flat, down) and an oval one.
' press: Pressed
' rapidr-env: RAPIDR_TEST_EVENTS=BtnPressed.__mousedown_5_5
CREATE Form AS QFORM
  Caption = "Buttons": Left = 40: Top = 40: Width = 340: Height = 220
  CREATE Ed AS QEDIT
    Left = 10: Top = 10: Width = 150: Text = "focus here"
  END CREATE
  CREATE BtnDefault AS QBUTTON
    Caption = "&Default": Left = 170: Top = 8: Default = 1
  END CREATE
  CREATE BtnNormal AS QBUTTON
    Caption = "Normal": Left = 10: Top = 44
  END CREATE
  CREATE BtnDisabled AS QBUTTON
    Caption = "Disabled": Left = 90: Top = 44: Enabled = 0
  END CREATE
  CREATE BtnPressed AS QBUTTON
    Caption = "Pressed": Left = 170: Top = 44
  END CREATE
  CREATE BtnTall AS QBUTTON
    Caption = "A tall button": Left = 250: Top = 44: Width = 70: Height = 50
  END CREATE
  CREATE Cool1 AS QCOOLBTN
    Caption = "Cool": Left = 10: Top = 84
  END CREATE
  CREATE Cool2 AS QCOOLBTN
    Caption = "Flat": Left = 90: Top = 84: Flat = 1
  END CREATE
  CREATE Cool3 AS QCOOLBTN
    Caption = "Down": Left = 170: Top = 84: GroupIndex = 1: Down = 1
  END CREATE
  CREATE Cool4 AS QCOOLBTN
    Caption = "Off": Left = 10: Top = 118: Enabled = 0
  END CREATE
  CREATE Oval AS QOVALBTN
    Caption = "Oval": Left = 90: Top = 118: Width = 75: Height = 30
  END CREATE
  CREATE BtnLong AS QBUTTON
    Caption = "A caption far too long": Left = 170: Top = 118
  END CREATE
END CREATE
Form.ShowModal

' Up-down controls (QUPDOWN: RapidQ's comes from an include library over
' Windows' msctls_updown32, updown.rapidq.bas makes those directly): a
' narrow one, a wide one, a horizontal one, each beside an edit.
CREATE Form AS QFORM
  Caption = "Up-down": Left = 40: Top = 40: Width = 260: Height = 120
  CREATE Ed1 AS QEDIT
    Left = 10: Top = 10: Width = 60: Text = "2"
  END CREATE
  CREATE Ud1 AS QUPDOWN
    Left = 72: Top = 9: Width = 17: Height = 24
  END CREATE
  CREATE Ud2 AS QUPDOWN
    Left = 100: Top = 9: Width = 40: Height = 24
  END CREATE
  CREATE Ud3 AS QUPDOWN
    Left = 150: Top = 10: Width = 34: Height = 17: Orientation = 1
  END CREATE
  CREATE Ud4 AS QUPDOWN
    Left = 10: Top = 44: Width = 17: Height = 40
  END CREATE
END CREATE
Form.ShowModal

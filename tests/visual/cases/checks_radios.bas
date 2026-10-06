' Check boxes and radio buttons in their states (off, on, disabled off and
' on), radio buttons in a group box and on the form; the first check box
' has the focus.
CREATE Form AS QFORM
  Caption = "Checks": Left = 40: Top = 40: Width = 330: Height = 230
  CREATE Chk1 AS QCHECKBOX
    Caption = "&Unchecked": Left = 10: Top = 10: Width = 120
  END CREATE
  CREATE Chk2 AS QCHECKBOX
    Caption = "Checked": Left = 10: Top = 32: Width = 120: Checked = 1
  END CREATE
  CREATE Chk3 AS QCHECKBOX
    Caption = "Disabled": Left = 10: Top = 54: Width = 120: Enabled = 0
  END CREATE
  CREATE Chk4 AS QCHECKBOX
    Caption = "Disabled on": Left = 10: Top = 76: Width = 120: Checked = 1: Enabled = 0
  END CREATE
  CREATE Grp AS QGROUPBOX
    Caption = "Shift": Left = 150: Top = 6: Width = 160: Height = 96
    CREATE RadDay AS QRADIOBUTTON
      Caption = "&Day": Left = 10: Top = 18: Width = 100: Checked = 1
    END CREATE
    CREATE RadNight AS QRADIOBUTTON
      Caption = "Night": Left = 10: Top = 40: Width = 100
    END CREATE
    CREATE RadOff AS QRADIOBUTTON
      Caption = "Disabled": Left = 10: Top = 62: Width = 100: Enabled = 0
    END CREATE
  END CREATE
  CREATE Rad1 AS QRADIOBUTTON
    Caption = "Form radio": Left = 10: Top = 110: Width = 120
  END CREATE
  CREATE Rad2 AS QRADIOBUTTON
    Caption = "Disabled on": Left = 10: Top = 132: Width = 120: Checked = 1: Enabled = 0
  END CREATE
  CREATE GrpOff AS QGROUPBOX
    Caption = "Disabled group": Left = 150: Top = 110: Width = 160: Height = 60: Enabled = 0
    CREATE ChkIn AS QCHECKBOX
      Caption = "Inside": Left = 10: Top = 24: Width = 100: Checked = 1
    END CREATE
  END CREATE
END CREATE
Form.ShowModal

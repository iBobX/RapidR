' A typical RapidQ dialog (tests/fixtures/a11y_form.bas without RapidR's
' own properties): labels sized for MS Sans Serif beside edits, a check
' box, radio buttons in a group box, a gauge, an up-down's place, a
' disabled panel with a button, a memo, OK / Cancel, a status bar.
CREATE Form AS QFORM
  Caption = "Sign in": Left = 40: Top = 40: Width = 400: Height = 300
  CREATE LblName AS QLABEL
    Caption = "&Name:": Left = 8: Top = 12: Width = 64
  END CREATE
  CREATE EdName AS QEDIT
    Left = 80: Top = 8: Width = 150: Text = "Ann"
  END CREATE
  CREATE LblPass AS QLABEL
    Caption = "Pass&word:": Left = 8: Top = 40: Width = 64
  END CREATE
  CREATE EdPass AS QEDIT
    Left = 80: Top = 36: Width = 150: PasswordChar = "*": Text = "abc"
  END CREATE
  CREATE ChkRemember AS QCHECKBOX
    Caption = "&Remember me": Left = 8: Top = 66: Width = 150
  END CREATE
  CREATE GrpShift AS QGROUPBOX
    Caption = "Shift": Left = 8: Top = 92: Width = 170: Height = 56
    CREATE RadDay AS QRADIOBUTTON
      Caption = "&Day": Left = 8: Top = 20: Width = 60: Checked = 1
    END CREATE
    CREATE RadNight AS QRADIOBUTTON
      Caption = "N&ight": Left = 80: Top = 20: Width = 70
    END CREATE
  END CREATE
  CREATE Upload AS QGAUGE
    Left = 8: Top = 156: Width = 170: Height = 18: Position = 50
  END CREATE
  CREATE PnlMore AS QPANEL
    Caption = "": Left = 60: Top = 182: Width = 120: Height = 30: Enabled = 0
    CREATE BtnMore AS QBUTTON
      Caption = "&More": Left = 2: Top = 2: Width = 80
    END CREATE
  END CREATE
  CREATE LblNotes AS QLABEL
    Caption = "Notes": Left = 200: Top = 66: Width = 80
  END CREATE
  CREATE MemNotes AS QRICHEDIT
    Left = 200: Top = 84: Width = 180: Height = 64
  END CREATE
  CREATE BtnOK AS QBUTTON
    Caption = "OK": Left = 200: Top = 182: Default = 1
  END CREATE
  CREATE BtnCancel AS QBUTTON
    Caption = "Cancel": Left = 290: Top = 182: Cancel = 1
  END CREATE
  CREATE SB AS QSTATUSBAR
    SimpleText = "Ready"
  END CREATE
END CREATE
Form.ShowModal

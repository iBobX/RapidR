' A focused push button (the first in the tab order: its dotted focus
' rectangle and its frame) beside a Default one and a Cancel one.
CREATE Form AS QFORM
  Caption = "Focus": Left = 40: Top = 40: Width = 280: Height = 90
  CREATE BtnFocus AS QBUTTON
    Caption = "&Focused": Left = 10: Top = 10
  END CREATE
  CREATE BtnDefault AS QBUTTON
    Caption = "OK": Left = 95: Top = 10: Default = 1
  END CREATE
  CREATE BtnCancel AS QBUTTON
    Caption = "Cancel": Left = 180: Top = 10: Cancel = 1
  END CREATE
END CREATE
Form.ShowModal

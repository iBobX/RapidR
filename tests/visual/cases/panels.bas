' Panels: the default (raised), lowered, both bevels, a wide bevel, a
' border, none, a single border style, a caption's alignments, a disabled
' one with a button in it.
CREATE Form AS QFORM
  Caption = "Panels": Left = 40: Top = 40: Width = 360: Height = 270
  CREATE P1 AS QPANEL
    Caption = "Default": Left = 10: Top = 10: Width = 100: Height = 50
  END CREATE
  CREATE P2 AS QPANEL
    Caption = "Lowered": Left = 120: Top = 10: Width = 100: Height = 50: BevelOuter = 1
  END CREATE
  CREATE P3 AS QPANEL
    Caption = "Frame": Left = 230: Top = 10: Width = 100: Height = 50: BevelOuter = 2: BevelInner = 1
  END CREATE
  CREATE P4 AS QPANEL
    Caption = "Wide": Left = 10: Top = 70: Width = 100: Height = 50: BevelWidth = 3
  END CREATE
  CREATE P5 AS QPANEL
    Caption = "Both lowered": Left = 120: Top = 70: Width = 100: Height = 50: BevelInner = 1: BevelOuter = 1
  END CREATE
  CREATE P6 AS QPANEL
    Caption = "None": Left = 230: Top = 70: Width = 100: Height = 50: BevelOuter = 0
  END CREATE
  CREATE P7 AS QPANEL
    Caption = "Single": Left = 10: Top = 130: Width = 100: Height = 50: BorderStyle = 1
  END CREATE
  CREATE P8 AS QPANEL
    Caption = "Left": Left = 120: Top = 130: Width = 100: Height = 24: Alignment = 0
  END CREATE
  CREATE P9 AS QPANEL
    Caption = "Right": Left = 120: Top = 156: Width = 100: Height = 24: Alignment = 1
  END CREATE
  CREATE P10 AS QPANEL
    Caption = "": Left = 230: Top = 130: Width = 110: Height = 50: Enabled = 0
    CREATE BtnIn AS QBUTTON
      Caption = "&More": Left = 4: Top = 4: Width = 80
    END CREATE
  END CREATE
  CREATE P11 AS QPANEL
    Caption = "Coloured": Left = 10: Top = 190: Width = 100: Height = 24: Color = &HC0FFFF
  END CREATE
  CREATE P12 AS QPANEL
    Caption = "Disabled": Left = 120: Top = 190: Width = 100: Height = 24: Enabled = 0
  END CREATE
END CREATE
Form.ShowModal

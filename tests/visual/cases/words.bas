' The words whose letters ran together in RapidQ's default font (RapidR
' Sans), each in a label of its own, regular and bold (a window caption's
' font): tools/visual/words.py puts them beside RapidQ's, word by word.
DIM Bold AS QFONT
Bold.AddStyles(fsBold)
CREATE Form AS QFORM
  Caption = "Words": Left = 40: Top = 40: Width = 300: Height = 150
  CREATE R1 AS QLABEL
    Caption = "program": Left = 8: Top = 8: Width = 120
  END CREATE
  CREATE R2 AS QLABEL
    Caption = "start": Left = 8: Top = 26: Width = 120
  END CREATE
  CREATE R3 AS QLABEL
    Caption = "Bread": Left = 8: Top = 44: Width = 120
  END CREATE
  CREATE R4 AS QLABEL
    Caption = "Price": Left = 8: Top = 62: Width = 120
  END CREATE
  CREATE R5 AS QLABEL
    Caption = "Right-click": Left = 8: Top = 80: Width = 120
  END CREATE
  CREATE R6 AS QLABEL
    Caption = "Notepad - untitled": Left = 8: Top = 98: Width = 120
  END CREATE
  CREATE B1 AS QLABEL
    Caption = "program": Left = 140: Top = 8: Width = 140
  END CREATE
  CREATE B2 AS QLABEL
    Caption = "start": Left = 140: Top = 26: Width = 140
  END CREATE
  CREATE B3 AS QLABEL
    Caption = "Bread": Left = 140: Top = 44: Width = 140
  END CREATE
  CREATE B4 AS QLABEL
    Caption = "Price": Left = 140: Top = 62: Width = 140
  END CREATE
  CREATE B5 AS QLABEL
    Caption = "Right-click": Left = 140: Top = 80: Width = 140
  END CREATE
  CREATE B6 AS QLABEL
    Caption = "Notepad - untitled": Left = 140: Top = 98: Width = 140
  END CREATE
END CREATE
B1.Font = Bold: B2.Font = Bold: B3.Font = Bold
B4.Font = Bold: B5.Font = Bold: B6.Font = Bold
Form.ShowModal

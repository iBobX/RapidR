' The gaps between letters in RapidQ's default font (RapidR Sans) where
' they are tightest — r after a round or upright letter, capitals before
' lower case — in a label, a bold label, an edit, a rich edit, a string
' grid and a status bar (the text editor and the label lay text out apart).
CREATE Form AS QFORM
  Caption = "Notepad - untitled": Left = 40: Top = 40: Width = 400: Height = 260
  CREATE L1 AS QLABEL
    Caption = "Br Pr Tr Fr Pa Te Wa Ty ra re ri ro rt ry rv rm tr fr pr ar": Left = 8: Top = 8: Width = 380
  END CREATE
  CREATE L2 AS QLABEL
    Caption = "Notepad - untitled  Bread  Price  program  start": Left = 8: Top = 28: Width = 380
    Font.AddStyles(fsBold)
  END CREATE
  CREATE E1 AS QEDIT
    Text = "Right-click me for the pop-up menu.": Left = 8: Top = 48: Width = 380
  END CREATE
  CREATE R1 AS QRICHEDIT
    Text = "This program is plain RapidQ code. Save it, start a new one": Left = 8: Top = 76: Width = 380: Height = 44
  END CREATE
  CREATE G1 AS QSTRINGGRID
    Left = 8: Top = 126: Width = 380: Height = 80: ColCount = 3: RowCount = 3
    Cell(0, 0) = "Item": Cell(1, 0) = "Price": Cell(2, 0) = "Qty"
    Cell(0, 1) = "Bread": Cell(1, 1) = "2.40": Cell(2, 1) = "3"
    Cell(0, 2) = "Tray": Cell(1, 2) = "Fresh": Cell(2, 2) = "Party"
  END CREATE
  CREATE SB AS QSTATUSBAR
    SimpleText = "Ready"
  END CREATE
END CREATE
Form.ShowModal

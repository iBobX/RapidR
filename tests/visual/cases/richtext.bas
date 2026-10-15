' Rich text in a QRICHEDIT: SelAttributes on parts of the text (bold,
' italic, underline, strike-out, colours, fonts, sizes); RapidQ's
' insertion format (a colour and bold set at the caret go to what is
' added there next); a second rich edit whose Alignment (taCenter) applies
' to the text set after it.
CREATE Form AS QFORM
  Caption = "Rich text": Left = 40: Top = 40: Width = 420: Height = 300
  CREATE Rich AS QRICHEDIT
    Left = 8: Top = 8: Width = 396: Height = 176
  END CREATE
  CREATE Centred AS QRICHEDIT
    Left = 8: Top = 192: Width = 396: Height = 68
  END CREATE
END CREATE

SUB Pick(W AS STRING)
  Rich.SelStart = INSTR(Rich.Text, W) - 1
  Rich.SelLength = LEN(W)
END SUB

CRLF$ = CHR$(13) + CHR$(10)
Rich.Text = "Rich text in RapidR" + CRLF$ + "Bold, italic, underline and strike-out." + CRLF$ + "Red, green and blue words." + CRLF$ + "Arial, Courier New and Times New Roman." + CRLF$ + "Small, normal and big."
Pick("Rich text in RapidR")
Rich.SelAttributes.Size = 14
Rich.SelAttributes.Bold = 1
Pick("Bold")
Rich.SelAttributes.Bold = 1
Pick("italic")
Rich.SelAttributes.Italic = 1
Pick("underline")
Rich.SelAttributes.Underline = 1
Pick("strike-out")
Rich.SelAttributes.StrikeOut = 1
Pick("Red")
Rich.SelAttributes.Color = &HFF
Pick("green")
Rich.SelAttributes.Color = &H8000
Pick("blue")
Rich.SelAttributes.Color = &HFF0000
Pick("Arial")
Rich.SelAttributes.Name = "Arial"
Pick("Courier New")
Rich.SelAttributes.Name = "Courier New"
Pick("Times New Roman")
Rich.SelAttributes.Name = "Times New Roman"
Pick("Small")
Rich.SelAttributes.Size = 6
Pick("big")
Rich.SelAttributes.Size = 16
Rich.SelStart = LEN(Rich.Text)
Rich.SelAttributes.Color = &H800080
Rich.SelAttributes.Bold = 1
Rich.AddStrings "Added at the caret: purple and bold."
Rich.SelStart = 0

Centred.Alignment = 2
Centred.Text = "Centred" + CRLF$ + "paragraphs"
Form.ShowModal

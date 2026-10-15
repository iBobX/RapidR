' QRICHEDIT's rich text, the same on every runtime: SelAttributes (its
' members, a whole QFONT, a colour set at the caret going to what
' AddStrings adds there), SelAttributes read back, Alignment, TextRTF
' (RapidR's: the RTF SaveToFile writes); the user typing inside a
' formatted word (the typed letters take its format) and Ctrl+Z (the
' text and its formats back).
DECLARE SUB Go
CREATE Form AS QFORM
  Caption = "rich"
  Width = 420: Height = 220
  CREATE Re AS QRICHEDIT
    Width = 300: Height = 110
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 120: Width = 400
  END CREATE
  CREATE Rtf AS QLABEL
    Top = 140: Visible = 0
  END CREATE
  CREATE Btn AS QBUTTON
    Top = 150
    OnClick = Go
  END CREATE
END CREATE
DIM F AS QFONT
F.Name = "Courier New"
F.Size = 12
F.Color = &HFF
F.AddStyles(fsBold)
Re.Text = "Hello world"
Re.SelStart = 6: Re.SelLength = 5
Re.SelAttributes = F
Re.SelStart = 0: Re.SelLength = 5
Re.SelAttributes.Italic = 1
Re.SelAttributes.Color = &HFF0000
Re.SelStart = LEN(Re.Text)
Re.SelAttributes.Color = &H8000
Re.SelAttributes.Underline = 1
Re.AddStrings "green"
Re.SelStart = 2
Form.ShowModal

SUB Go
  T$ = Re.Text
  P = Re.SelStart
  Re.SelStart = 2: Re.SelLength = 2
  A$ = HEX$(Re.SelAttributes.Color) + Re.SelAttributes.Name + STR$(Re.SelAttributes.Italic) + STR$(Re.SelAttributes.Size)
  Re.SelStart = 14: Re.SelLength = 0
  B$ = HEX$(Re.SelAttributes.Color) + STR$(Re.SelAttributes.Underline) + STR$(Re.SelAttributes.Bold)
  Re.SelStart = P
  Lbl.Caption = Lbl.Caption + "|" + REPLACESUBSTR$(T$, CHR$(13) + CHR$(10), "/") + "," + STR$(P) + "," + A$ + "," + B$
  Rtf.Caption = REPLACESUBSTR$(REPLACESUBSTR$(Re.TextRTF, CHR$(13), ""), CHR$(10), "~")
END SUB

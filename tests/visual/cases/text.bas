' Labels (default, disabled, alignments, a & letter, a coloured one) and
' text boxes: an edit with the focus, a disabled one, a read-only one, a
' password; two rich edits (RapidQ has no QMEMO) with lines.
CREATE Form AS QFORM
  Caption = "Text": Left = 40: Top = 40: Width = 380: Height = 290
  CREATE EdFocus AS QEDIT
    Left = 90: Top = 8: Width = 140: Text = "Focused"
  END CREATE
  CREATE Lbl1 AS QLABEL
    Caption = "&Name:": Left = 10: Top = 12
  END CREATE
  CREATE Lbl2 AS QLABEL
    Caption = "Password:": Left = 10: Top = 40
  END CREATE
  CREATE EdPass AS QEDIT
    Left = 90: Top = 36: Width = 140: PasswordChar = "*": Text = "secret"
  END CREATE
  CREATE Lbl3 AS QLABEL
    Caption = "Disabled:": Left = 10: Top = 68: Enabled = 0
  END CREATE
  CREATE EdOff AS QEDIT
    Left = 90: Top = 64: Width = 140: Text = "Disabled": Enabled = 0
  END CREATE
  CREATE Lbl4 AS QLABEL
    Caption = "Read-only:": Left = 10: Top = 96
  END CREATE
  CREATE EdRo AS QEDIT
    Left = 90: Top = 92: Width = 140: Text = "Read only": ReadOnly = 1
  END CREATE
  CREATE LblR AS QLABEL
    Caption = "Right": Left = 240: Top = 12: Width = 120: Alignment = 1: AutoSize = 0
  END CREATE
  CREATE LblC AS QLABEL
    Caption = "Centred": Left = 240: Top = 30: Width = 120: Alignment = 2: AutoSize = 0
  END CREATE
  CREATE LblColor AS QLABEL
    Caption = "On colour": Left = 240: Top = 50: Width = 120: AutoSize = 0: Color = &HFFC0C0
  END CREATE
  CREATE LblFont AS QLABEL
    Caption = "Red text": Left = 240: Top = 70
  END CREATE
  CREATE Memo AS QRICHEDIT
    Left = 10: Top = 124: Width = 170: Height = 80
  END CREATE
  CREATE Rich AS QRICHEDIT
    Left = 190: Top = 124: Width = 170: Height = 80
  END CREATE
  CREATE LblLong AS QLABEL
    Caption = "The quick brown fox jumps over the lazy dog. 0123456789": Left = 10: Top = 214
  END CREATE
  CREATE LblWrap AS QLABEL
    Caption = "Word wrap: the quick brown fox jumps over the lazy dog.": Left = 10: Top = 232: Width = 200: WordWrap = 1: AutoSize = 0: Height = 30
  END CREATE
END CREATE
LblFont.Font.Color = &HFF
Memo.Text = "Line one" + CHR$(13) + CHR$(10) + "Line two" + CHR$(13) + CHR$(10) + "Line three"
Rich.Text = "Rich edit" + CHR$(13) + CHR$(10) + "Second line"
Form.ShowModal

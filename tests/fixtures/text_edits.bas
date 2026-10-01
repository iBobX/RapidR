' QEDIT / QRICHEDIT keep their text and selection in one model (shared by
' every runtime): read before the form shows, SelStart / SelLength /
' SelText, Line(i), LineCount, AddStrings, Modified, WhereY.
DECLARE SUB Go
CREATE Form AS QFORM
  Caption = "edits"
  CREATE Ed AS QEDIT
    Text = "hello"
  END CREATE
  CREATE Re AS QRICHEDIT
    Top = 30: Width = 200: Height = 100
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 140: Width = 400
  END CREATE
  CREATE Btn AS QBUTTON
    Top = 170
    OnClick = Go
  END CREATE
END CREATE
Re.AddStrings "one", "two"
Lbl.Caption = STR$(Re.LineCount) + Re.Line(1)
Form.ShowModal

SUB Go
  Ed.SelStart = 1: Ed.SelLength = 3
  S$ = Ed.SelText
  Ed.SelText = "EY"
  Re.Line(0) = "ONE"
  Re.SelStart = 4: Re.SelLength = 3
  T$ = Re.SelText
  Re.SelText = "2"
  Lbl.Caption = Lbl.Caption + "|" + S$ + "|" + Ed.Text + "|" + STR$(Ed.SelStart) + "|" + T$ + "|" + Re.Line(0) + "|" + Re.Line(1) + "|" + STR$(LEN(Re.Text)) + "|" + STR$(Re.Modified) + "|" + STR$(Re.WhereY)
END SUB

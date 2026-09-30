' QOUTLINE (the manual's example): lines added with AddLines, each leading
' space a level deeper; AddChild(Index, S), Item(i) read and written, Row,
' LineCount, OnClick — shown as a tree.
DECLARE SUB Clicked
DECLARE SUB Report
DIM Clicks AS INTEGER
CREATE Form AS QForm
  Caption = "outline"
  Width = 360
  Height = 260
  CREATE OutLine AS QOutLine
    Left = 5 : Top = 5 : Width = 200 : Height = 180
    AddLines "Parent 1", _
             " Child of Parent 1", _
             "  Child of Child of Parent 1", _
             "Parent 2", _
             "Parent 3"
    OnClick = Clicked
  END CREATE
  CREATE Btn AS QBUTTON
    Left = 220 : Top = 5
    Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 5 : Top = 195 : Width = 340
    Caption = "-"
  END CREATE
END CREATE
OutLine.AddChild 3, "Child of Parent 2"
OutLine.Item(0) = "First"
SUB Clicked
  Clicks = Clicks + 1
END SUB
SUB Report
  Lbl.Caption = STR$(OutLine.LineCount) + " " + OutLine.Item(0) + " " + OutLine.Item(OutLine.Row) + " " + STR$(Clicks)
END SUB
Form.ShowModal

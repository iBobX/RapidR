' QSTRINGGRID: VisibleRowCount / VisibleColCount (the rows / columns
' shown whole in the control) and moving columns and rows — by the mouse
' with goColMoving / goRowMoving (a header cell dragged), and RapidR's
' MoveCol / MoveRow from the program.
$INCLUDE "RAPIDQ.INC"
DECLARE SUB Report
CREATE Form AS QFORM
  Caption = "grid moving": Width = 360: Height = 260
  CREATE Grid AS QSTRINGGRID
    Width = 300: Height = 160
    RowCount = 10
    AddOptions goColMoving, goRowMoving
  END CREATE
  CREATE Btn AS QBUTTON
    Top = 170 : Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 200: Width = 340
  END CREATE
END CREATE
FOR C = 0 TO 4
  Grid.Cell(C, 0) = "H" + STR$(C)
  Grid.Cell(C, 1) = "a" + STR$(C)
NEXT
Grid.Cell(0, 2) = "R2"
Grid.ColWidths(1) = 40

SUB Report
  DIM Seen AS STRING
  Seen = STR$(Grid.VisibleRowCount) + " " + STR$(Grid.VisibleColCount)
  Grid.MoveCol 1, 3
  Grid.MoveRow 2, 4
  Lbl.Caption = Seen + " " + Grid.Cell(3, 0) + Grid.Cell(3, 1) + " " + STR$(Grid.ColWidths(3)) + " " + Grid.Cell(0, 4) + " " + STR$(Grid.VisibleColCount)
END SUB

Form.ShowModal

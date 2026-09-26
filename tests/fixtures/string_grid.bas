' QSTRINGGRID cells, sizes, rows/columns and Separator-based streams, checked
' natively and interpreted by tests/native_gui_events.mjs (the web runtime
' draws the same data: tests/web_ide_grid.mjs).
DECLARE SUB Go
CREATE Form AS QFORM
  Caption = "Grid": Width = 420: Height = 260
  CREATE Grid AS QSTRINGGRID
    Width = 400: Height = 170
    ColCount = 4: RowCount = 5
    Cell(1, 0) = "Name": Cell(2, 0) = "Age": Cell(3, 0) = "City"
    ColWidths(0) = 30
    AddOptions 10
  END CREATE
  CREATE Btn AS QBUTTON
    Top = 180: Caption = "Go": OnClick = Go
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 210: Width = 400
  END CREATE
END CREATE
DIM i AS INTEGER
FOR i = 1 TO 4
  Grid.Cell(0, i) = STR$(i)
  Grid.Cell(1, i) = "P" + STR$(i)
  Grid.Cell(2, i) = STR$(20 + i)
NEXT
Grid.Cell(3, 2) = "Lima"

SUB Go
  DIM s AS QMEMORYSTREAM, copy AS QSTRINGGRID
  Grid.InsertRow 2
  Grid.SwapRows 1, 3
  Grid.DeleteCol 0
  Grid.Separator = ";"
  Grid.SaveToStream s, 1, 0, 3
  s.Position = 0
  copy.Separator = ";"
  copy.RowCount = 1
  copy.LoadFromStream s, 1, 0, 0
  Grid.Row = 4: Grid.Col = 1
  Lbl.Caption = Grid.Cell(0, 1) + "|" + Grid.Cell(0, 3) + "|" + Grid.Cell(2, 1) + "|" + STR$(Grid.RowCount) + "|" + STR$(Grid.ColCount) + "|" + STR$(Grid.ColWidths(0)) + "|" + copy.Cell(0, 3) + "|" + copy.Cell(2, 1) + "|" + STR$(copy.RowCount) + "|" + STR$(Grid.Row) + STR$(Grid.Col) + "|" + STR$(Grid.EditorMode)
END SUB

Form.ShowModal

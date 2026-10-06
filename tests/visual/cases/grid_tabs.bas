' A string grid (a fixed row and column, cells, the current cell) and a
' tab control with three tabs, the second chosen.
CREATE Form AS QFORM
  Caption = "Grid and tabs": Left = 40: Top = 40: Width = 400: Height = 300
  CREATE Grid AS QSTRINGGRID
    Left = 10: Top = 10: Width = 370: Height = 120
    ColCount = 4: RowCount = 5
  END CREATE
  CREATE Tabs AS QTABCONTROL
    Left = 10: Top = 140: Width = 370: Height = 110
  END CREATE
END CREATE
Grid.Cell(1, 0) = "Name"
Grid.Cell(2, 0) = "Age"
Grid.Cell(3, 0) = "City"
DIM i AS INTEGER
FOR i = 1 TO 4
  Grid.Cell(0, i) = STR$(i)
  Grid.Cell(1, i) = "Person " + STR$(i)
  Grid.Cell(2, i) = STR$(20 + i)
NEXT
Grid.Cell(3, 2) = "Lima"
Tabs.AddTabs "General", "Advanced", "About"
Tabs.TabIndex = 1
Form.ShowModal

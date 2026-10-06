' A list view in report style (column headers, items, sub-items, a
' selected row with the focus) and one with grid lines and row select.
CREATE Form AS QFORM
  Caption = "List view": Left = 40: Top = 40: Width = 380: Height = 260
  CREATE LV AS QLISTVIEW
    Left = 10: Top = 10: Width = 350: Height = 100: ViewStyle = 3
  END CREATE
  CREATE LV2 AS QLISTVIEW
    Left = 10: Top = 120: Width = 350: Height = 90: ViewStyle = 3: GridLines = 1: RowSelect = 1
  END CREATE
END CREATE
LV.AddColumns "Name", "Size", "Kind"
LV.Column(0).Width = 140
LV.AddItems "readme.txt", "notes.bas", "picture.bmp"
LV.AddSubItem 0, "12 KB"
LV.AddSubItem 0, "Text"
LV.AddSubItem 1, "3 KB"
LV.AddSubItem 1, "BASIC"
LV.AddSubItem 2, "640 KB"
LV.AddSubItem 2, "Bitmap"
LV.ItemIndex = 1
LV2.AddColumns "Col A", "Col B"
LV2.AddItems "One", "Two", "Three"
LV2.AddSubItem 0, "1"
LV2.AddSubItem 1, "2"
LV2.AddSubItem 2, "3"
LV2.ItemIndex = 0
Form.ShowModal

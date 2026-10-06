' A list box with a selection (focused), a disabled one, combo boxes:
' a drop-down with text, a drop-down list, a disabled one.
CREATE Form AS QFORM
  Caption = "Lists": Left = 40: Top = 40: Width = 360: Height = 220
  CREATE List1 AS QLISTBOX
    Left = 10: Top = 10: Width = 120: Height = 100
  END CREATE
  CREATE List2 AS QLISTBOX
    Left = 140: Top = 10: Width = 80: Height = 60: Enabled = 0
  END CREATE
  CREATE Combo1 AS QCOMBOBOX
    Left = 230: Top = 10: Width = 110
  END CREATE
  CREATE Combo2 AS QCOMBOBOX
    Left = 230: Top = 40: Width = 110: Style = 2
  END CREATE
  CREATE Combo3 AS QCOMBOBOX
    Left = 230: Top = 70: Width = 110: Enabled = 0
  END CREATE
  CREATE List3 AS QLISTBOX
    Left = 10: Top = 120: Width = 330: Height = 56: Columns = 3
  END CREATE
END CREATE
DIM i AS INTEGER
FOR i = 1 TO 9
  List1.AddItems "Item " + STR$(i)
  List3.AddItems "Column item " + STR$(i)
NEXT
List1.ItemIndex = 2
List2.AddItems "One", "Two"
Combo1.AddItems "Apple", "Banana", "Cherry"
Combo1.Text = "Banana"
Combo2.AddItems "Red", "Green", "Blue"
Combo2.ItemIndex = 1
Combo3.AddItems "Off"
Combo3.ItemIndex = 0
Form.ShowModal

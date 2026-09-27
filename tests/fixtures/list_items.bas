' QLISTBOX / QCOMBOBOX items (Item, ItemCount, ItemIndex, AddItems,
' InsertItem, DelItems, Sorted, MultiSelect, Selected, Text), checked
' natively and interpreted by tests/native_gui_events.mjs and in the browser
' by tests/web_ide_lists.mjs.
DECLARE SUB Picked
DECLARE SUB Changed
CREATE Form AS QFORM
  Caption = "Lists": Width = 340: Height = 220
  CREATE Items AS QLISTBOX
    Width = 150: Height = 120
    AddItems "one", "two", "@b not bold", "four"
    OnClick = Picked
  END CREATE
  CREATE Combo AS QCOMBOBOX
    Left = 170: Width = 150
    AddItems "red", "a/b & c", "blue"
    OnChange = Changed
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 130: Width = 320
  END CREATE
  CREATE Summary AS QLABEL
    Top = 155: Width = 320
  END CREATE
END CREATE
DIM Fruit AS QLISTBOX
Items.AddItems "five"
Items.InsertItem 0, "zero"
Items.DelItems 2
Items.Item(1) = "ONE"
Items.ItemIndex = 3
Combo.ItemIndex = 1
Fruit.AddItems "pear", "Apple", "fig"
Fruit.ItemIndex = 0
Fruit.Sorted = 1
Fruit.MultiSelect = 1
Fruit.Selected(0) = 1
Summary.Caption = STR$(Items.ItemCount) + "|" + Items.Item(0) + "|" + Items.Item(Items.ItemIndex) + "|" + Combo.Text + "|" + STR$(Combo.ItemCount) + "|" + Fruit.Item(0) + Fruit.Item(2) + "|" + STR$(Fruit.ItemIndex) + "|" + STR$(Fruit.SelCount)

SUB Picked
  Lbl.Caption = "picked " + STR$(Items.ItemIndex) + " " + Items.Item(Items.ItemIndex)
END SUB

SUB Changed
  Lbl.Caption = "combo " + STR$(Combo.ItemIndex) + " " + Combo.Text
END SUB

Form.ShowModal

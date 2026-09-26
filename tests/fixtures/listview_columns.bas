' QLISTVIEW columns, items and sub-items (RapidQ's ZIP viewer style),
' checked natively and interpreted by tests/native_gui_events.mjs.
DECLARE SUB Clicked
DECLARE SUB Headed (Column AS INTEGER)
CREATE Form AS QFORM
  Caption = "ListView": Width = 420: Height = 220
  CREATE LV AS QLISTVIEW
    Width = 400: Height = 150
    ViewStyle = 3
    AddColumns "FileName", "Size", "Method"
    Column(0).Width = 200
    OnClick = Clicked
    OnColumnClick = Headed
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 160: Width = 400
  END CREATE
END CREATE
LV.AddItems "readme.txt", "photo", "data.bin"
LV.AddSubItem 0, "1200"
LV.AddSubItem 0, "Stored"
LV.AddSubItem 2, "88"
LV.AddSubItem 2, "Deflated"
LV.Item(1).Caption = "photo.jpg"
LV.AddItems "@b not bold"
LV.InsertItem 1, "first.txt"
LV.ItemIndex = 2

SUB Clicked
  Lbl.Caption = STR$(LV.ItemIndex) + "|" + LV.Item(LV.ItemIndex).Caption + "|" + LV.SubItem(3, 1) + "|" + STR$(LV.ItemCount) + "|" + STR$(LV.ColumnsCount) + "|" + STR$(LV.Column(0).Width) + "|" + LV.Column(2).Caption
END SUB

SUB Headed (Column AS INTEGER)
  Lbl.Caption = "column" + STR$(Column)
END SUB

Form.ShowModal

' QLISTVIEW item picks by the test hooks (`lv.__item_N`, as a list's and a
' tree's): item 1, then item 10 — below the control's fold, brought into
' view — each answered by OnClick with the item picked. Checked natively and
' interpreted by tests/native_gui_events.mjs and in the browser.
DECLARE SUB Clicked
CREATE Form AS QFORM
  Caption = "ListView picks": Width = 300: Height = 220
  CREATE LV AS QLISTVIEW
    Width = 280: Height = 120
    ViewStyle = 3
    AddColumns "Name", "Size"
    Column(0).Width = 160
    OnClick = Clicked
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 130: Width = 280
  END CREATE
END CREATE
DIM i AS INTEGER
FOR i = 0 TO 11
  LV.AddItems "item " + STR$(i)
NEXT
LV.ItemIndex = 0

SUB Clicked
  Lbl.Caption = Lbl.Caption + STR$(LV.ItemIndex) + ":" + LV.Item(LV.ItemIndex).Caption + "|"
END SUB

Form.ShowModal

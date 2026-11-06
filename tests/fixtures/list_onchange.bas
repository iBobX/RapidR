' RListBox's OnChange (RapidR's: RapidQ's QLISTBOX has none — RC.EXE says
' "Member ONCHANGE not part of class") fires whenever the user moves the
' selection to another item: a click on another item, as the arrow keys,
' Page Up / Down, Home and End do (RAPIDR-ISSUES #9: the click didn't), each
' before its OnClick; a click on the item already selected fires OnClick
' only. tests/gui_parity_cases.mjs ("list_onchange").
DECLARE SUB Picked
DECLARE SUB Clicked
DIM Trail AS STRING
CREATE F AS RForm
  Caption = "list OnChange"
  Width = 320
  Height = 240
  CREATE L AS RListBox
    Left = 10
    Top = 10
    Width = 150
    Height = 120
    OnChange = Picked
    OnClick = Clicked
  END CREATE
  CREATE Lk AS RLabel
    Left = 10
    Top = 150
    Width = 300
    Caption = "-"
  END CREATE
END CREATE
SUB Picked
  Trail = Trail + " change" + STR$(L.ItemIndex)
  Lk.Caption = Trail
END SUB
SUB Clicked
  Trail = Trail + " click" + STR$(L.ItemIndex)
  Lk.Caption = Trail
END SUB
L.AddItems "one", "two", "three", "four"
F.ShowModal

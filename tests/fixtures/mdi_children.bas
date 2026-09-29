' QFORMMDI (RAPIDQ2.INC): child windows each showing a component —
' AddChild by Handle, OnChildActive / OnChildClose (ChildResult keeps a
' child open), tiling, next child, CloseChild / CloseAllChild, FreeChild,
' GetChild, ChildExist, the component placed inside its frame.
$INCLUDE "RAPIDQ.INC"
DIM ed(3) AS QRICHEDIT
DIM log AS STRING

SUB Active (h AS LONG, i AS INTEGER, t AS STRING)
  log = log + "A" + STR$(i) + " "
END SUB

SUB Closing (h AS LONG, i AS INTEGER, t AS STRING)
  log = log + "C" + STR$(i) + " "
  IF i = 1 THEN Form.ChildResult = 0
END SUB

SUB Show
  lbl.Caption = log + "|n" + STR$(Form.ChildCount) + "|" + Form.ChildCaption + "|i" + STR$(Form.ComponentIndex) + "|free" + STR$(Form.FreeChild(ed(0).Handle)) + STR$(Form.FreeChild(ed(3).Handle)) + "|get" + STR$(Form.GetChild("Two")) + STR$(Form.ChildExist("Nope"))
  geo.Caption = STR$(ed(1).Left) + "," + STR$(ed(1).Top) + "," + STR$(ed(1).Width) + "," + STR$(ed(1).Height) + "|" + STR$(Form.ChildWidth) + "|vis" + STR$(ed(0).Visible <> 0) + STR$(ed(1).Visible <> 0)
END SUB

SUB AddThem
  Form.AddChild(ed(0).Handle, "One", 0, 0, 0, 0, 0, 1)
  Form.AddChild(ed(1).Handle, "Two", 1, 0, 0, 0, 0, 1)
  Form.AddChild(ed(2).Handle, "Three", 2, 0, 0, 0, 0, 1)
  Show
END SUB

SUB TileThem
  Form.SetVertChild
  Form.ActiveNextChild
  Show
END SUB

SUB CloseOne
  Form.CloseChild
  Show
END SUB

SUB CloseAll
  Form.CloseAllChild
  Show
END SUB

CREATE Form AS QFORMMDI
  Caption = "MDI"
  Width = 620: Height = 420
  OnChildActive = Active
  OnChildClose = Closing
  CREATE bAdd AS QBUTTON
    Caption = "Add": Left = 0: Top = 330: OnClick = AddThem
  END CREATE
  CREATE bTile AS QBUTTON
    Caption = "Tile": Left = 80: Top = 330: OnClick = TileThem
  END CREATE
  CREATE bClose AS QBUTTON
    Caption = "Close": Left = 160: Top = 330: OnClick = CloseOne
  END CREATE
  CREATE bAll AS QBUTTON
    Caption = "All": Left = 240: Top = 330: OnClick = CloseAll
  END CREATE
  CREATE lbl AS QLABEL
    Left = 0: Top = 360: Width = 600
  END CREATE
  CREATE geo AS QLABEL
    Left = 0: Top = 378: Width = 600
  END CREATE
END CREATE
FOR k = 0 TO 3
  ed(k).Parent = Form
  ed(k).Visible = False
NEXT
Form.ShowModal

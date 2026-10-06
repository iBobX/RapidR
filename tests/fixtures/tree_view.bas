' QTREEVIEW: nodes numbered depth-first, Item(i)'s
' Text / Count / Level / Expanded / HasChildren, and the events that answer
' back — OnChanging's AllowChange, OnExpanding's AllowExpansion — then
' OnChange / OnExpanded; DelItems takes a node's subtree (OnDeletion).
DECLARE SUB Changing (Node AS INTEGER, AllowChange AS INTEGER)
DECLARE SUB Changed (Node AS INTEGER)
DECLARE SUB Expanding (Node AS INTEGER, AllowExpansion AS INTEGER)
DECLARE SUB Expanded (Node AS INTEGER)
DECLARE SUB Deleted (Node AS INTEGER)
DECLARE SUB Report
DIM Log AS STRING, Dels AS STRING
CREATE Form AS QFORM
  Caption = "tree view"
  Width = 360
  Height = 300
  CREATE Tv AS QTREEVIEW
    Left = 5
    Top = 5
    Width = 200
    Height = 200
    AddItems "North", "South", "East"
    AddChildItems 0, "Hill", "Lake", "Wood"
    AddChildItems 4, "Port", "Bay"
    OnChanging = Changing
    OnChange = Changed
    OnExpanding = Expanding
    OnExpanded = Expanded
    OnDeletion = Deleted
  END CREATE
  CREATE Btn AS QBUTTON
    Left = 220
    Top = 5
    Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 5
    Top = 210
    Width = 340
    Caption = "-"
  END CREATE
  CREATE Lbl2 AS QLABEL
    Left = 5
    Top = 235
    Width = 340
    Caption = "-"
  END CREATE
END CREATE
SUB Changing (Node AS INTEGER, AllowChange AS INTEGER)
  IF Node = 2 THEN AllowChange = 0
END SUB
SUB Changed (Node AS INTEGER)
  Log = Log + "chg" + STR$(Node) + " "
END SUB
SUB Expanding (Node AS INTEGER, AllowExpansion AS INTEGER)
  IF Node = 4 THEN AllowExpansion = 0
END SUB
SUB Expanded (Node AS INTEGER)
  Log = Log + "exp" + STR$(Node) + " "
END SUB
SUB Deleted (Node AS INTEGER)
  Dels = Dels + "del" + STR$(Node) + " "
  Lbl2.Caption = Dels + STR$(Tv.ItemCount)
END SUB
SUB Report
  Lbl.Caption = Log + "|" + STR$(Tv.ItemCount) + "|" + Tv.Item(Tv.ItemIndex).Text + "|" + STR$(Tv.Item(0).Count) + STR$(Tv.Item(1).Level) + STR$(Tv.Item(0).Expanded) + STR$(Tv.Item(4).Expanded) + STR$(Tv.Item(4).HasChildren)
  Tv.DelItems 4
END SUB
Form.ShowModal

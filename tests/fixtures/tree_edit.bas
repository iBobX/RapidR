' QTREEVIEW in-place editing: F2 (or a click on the selected node) asks
' OnEditing (Index, AllowEdit) — which may refuse — then Enter asks
' OnEdited (Index, S), whose S answers back the node's text; Escape drops
' the edit and ReadOnly allows none.
DECLARE SUB Editing (Node AS INTEGER, AllowEdit AS INTEGER)
DECLARE SUB Edited (Node AS INTEGER, S AS STRING)
DECLARE SUB Lock
DECLARE SUB Report
DIM Log AS STRING
CREATE Form AS QFORM
  Caption = "tree edit"
  Width = 360
  Height = 300
  CREATE Tv AS QTREEVIEW
    Left = 5
    Top = 5
    Width = 200
    Height = 200
    AddItems "Apple", "Pear", "Plum"
    OnEditing = Editing
    OnEdited = Edited
  END CREATE
  CREATE Ro AS QBUTTON
    Left = 220
    Top = 5
    Caption = "Lock"
    OnClick = Lock
  END CREATE
  CREATE Btn AS QBUTTON
    Left = 220
    Top = 40
    Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 5
    Top = 210
    Width = 340
    Caption = "-"
  END CREATE
END CREATE
SUB Editing (Node AS INTEGER, AllowEdit AS INTEGER)
  Log = Log + "ing" + STR$(Node) + " "
  IF Node = 2 THEN AllowEdit = 0
END SUB
SUB Edited (Node AS INTEGER, S AS STRING)
  Log = Log + "ed" + STR$(Node) + ":" + S + " "
  S = UCASE$(S)
END SUB
SUB Lock
  Tv.ReadOnly = 1
END SUB
SUB Report
  Lbl.Caption = Log + "|" + Tv.Item(0).Text + " " + Tv.Item(1).Text + " " + Tv.Item(2).Text + " " + STR$(Tv.ReadOnly)
END SUB
Form.ShowModal

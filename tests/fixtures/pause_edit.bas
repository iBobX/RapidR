' Windows' edit after a pause: a click on the selected node of a focused
' QTREEVIEW, or on the selected item of a focused QLISTVIEW, edits its
' text once the double-click time is up (a tree asks OnEditing first) —
' unless a double click comes first (OnDblClick, and no edit).
$INCLUDE "RAPIDQ.INC"
DECLARE SUB Editing (Node AS INTEGER, AllowEdit AS INTEGER)
DECLARE SUB Edited (Node AS INTEGER, S AS STRING)
DECLARE SUB Changed (Index AS INTEGER, Change AS BYTE)
DECLARE SUB LvDbl
DECLARE SUB Report
DIM Log AS STRING
CREATE Form AS QFORM
  Caption = "pause edit": Width = 460: Height = 300
  CREATE Tv AS QTREEVIEW
    Left = 5: Top = 5: Width = 150: Height = 150
    AddItems "Apple", "Pear", "Plum"
    OnEditing = Editing
    OnEdited = Edited
  END CREATE
  CREATE LV AS QLISTVIEW
    Left = 170: Top = 5: Width = 200: Height = 150
    ViewStyle = vsReport
    AddColumns "Name"
    Column(0).Width = 120
    AddItems "one", "two", "three"
    OnChange = Changed
    OnDblClick = LvDbl
  END CREATE
  CREATE Btn AS QBUTTON
    Left = 5: Top = 170: Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 5: Top = 210: Width = 440: Caption = "-"
  END CREATE
END CREATE
SUB Editing (Node AS INTEGER, AllowEdit AS INTEGER)
  Log = Log + "ing" + STR$(Node) + " "
END SUB
SUB Edited (Node AS INTEGER, S AS STRING)
  Log = Log + "ed" + STR$(Node) + ":" + S + " "
END SUB
SUB Changed (Index AS INTEGER, Change AS BYTE)
  Log = Log + "c" + STR$(Index) + ":" + STR$(Change) + " "
END SUB
SUB LvDbl
  Log = Log + "dbl "
END SUB
SUB Report
  Lbl.Caption = Log + "|" + Tv.Item(1).Text + " " + LV.Item(0).Caption + " " + LV.Item(1).Caption
END SUB
Form.ShowModal

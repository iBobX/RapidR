' Inherit<Event> (manual 10.4): a TYPE's EVENT OnClick, replaced by the
' program's own handler, still runs when that handler inherits it.
TYPE Clicker EXTENDS QBUTTON
  Count AS INTEGER
  EVENT OnClick
    This.Count = This.Count + 1
    Log = Log + "own "
  END EVENT
END TYPE
DIM Log AS STRING
CREATE Form AS QFORM
  Caption = "inherit"
  CREATE C AS Clicker
    Caption = "Click"
  END CREATE
  CREATE Plain AS Clicker
    Top = 30 : Caption = "Plain"
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 60 : Width = 300
  END CREATE
  CREATE Btn AS QBUTTON
    Top = 90 : Caption = "Report"
  END CREATE
END CREATE
SUB Mine
  C.InheritOnClick
  Log = Log + "mine" + STR$(C.Count) + " "
END SUB
SUB Report
  Lbl.Caption = Log + "| " + STR$(Plain.Count)
END SUB
C.OnClick = Mine
Btn.OnClick = Report
Form.ShowModal

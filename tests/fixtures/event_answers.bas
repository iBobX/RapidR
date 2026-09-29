' Event parameters come back to the runtime (RapidQ passes them by
' reference): OnClose's Action (caNone keeps the form), QSTRINGGRID
' OnSelectCell's CanSelect (0 refuses the cell, also from a TYPE's EVENT),
' QLISTBOX OnMeasureItem's Height (lbOwnerDrawVariable).
$INCLUDE "RAPIDQ.INC"
DECLARE SUB ShowDialogs
DECLARE SUB CloseByCode
DECLARE SUB KeepOpen (Action AS INTEGER)
DECLARE SUB LetClose (Action AS INTEGER, Sender AS QFORM)
DECLARE SUB PickCell (Col AS INTEGER, Row AS INTEGER, CanSelect AS INTEGER)
DECLARE SUB Measure (Index AS INTEGER, Height AS INTEGER)
DECLARE SUB DrawItem (Index AS INTEGER, State AS INTEGER, R AS QRECT)
DIM Log AS STRING

TYPE TGuardGrid EXTENDS QSTRINGGRID
  Refused AS INTEGER
  EVENT OnSelectCell (C AS INTEGER, R AS INTEGER, CanSelect AS INTEGER)
    IF R = 2 THEN CanSelect = 0: Refused = Refused + 1
  END EVENT
END TYPE

CREATE Form AS QFORM
  Caption = "event answers"
  Width = 480
  Height = 330
  CREATE Grid AS QSTRINGGRID
    Left = 5
    Top = 5
    Width = 220
    Height = 90
    ColCount = 4
    RowCount = 4
    OnSelectCell = PickCell
  END CREATE
  CREATE Lst AS QLISTBOX
    Left = 5
    Top = 100
    Width = 150
    Height = 120
    Style = lbOwnerDrawVariable
    ItemHeight = 16
    OnMeasureItem = Measure
    OnDrawItem = DrawItem
    AddItems "one", "two", "three"
  END CREATE
  CREATE Show AS QBUTTON
    Left = 240
    Top = 5
    Caption = "Show"
    OnClick = ShowDialogs
  END CREATE
  CREATE Code AS QBUTTON
    Left = 240
    Top = 35
    Caption = "Close"
    OnClick = CloseByCode
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 5
    Top = 230
    Width = 460
    Caption = "-"
  END CREATE
  CREATE Lbl2 AS QLABEL
    Left = 5
    Top = 255
    Width = 460
    Caption = "-"
  END CREATE
END CREATE

DIM G2 AS TGuardGrid
G2.Parent = Form
G2.Left = 240
G2.Top = 100
G2.Width = 220
G2.Height = 90
G2.ColCount = 4
G2.RowCount = 4

CREATE Dlg AS QFORM
  Caption = "keeps open"
  Width = 200
  Height = 100
  OnClose = KeepOpen
END CREATE
CREATE Dlg2 AS QFORM
  Caption = "lets close"
  Width = 200
  Height = 100
  OnClose = LetClose
END CREATE

SUB ShowDialogs
  Dlg.Show
  Dlg2.Show
END SUB

SUB CloseByCode
  Dlg.Close
  Log = Log + "code "
  Lbl.Caption = Log + "|" + STR$(Grid.Col) + "," + STR$(Grid.Row) + "|" + STR$(G2.Col) + "," + STR$(G2.Row) + "," + STR$(G2.Refused)
END SUB

SUB KeepOpen (Action AS INTEGER)
  Log = Log + "keep" + STR$(Action) + " "
  Action = caNone
END SUB

SUB LetClose (Action AS INTEGER, Sender AS QFORM)
  Log = Log + "let" + STR$(Action) + Sender.Caption + " "
END SUB

SUB PickCell (Col AS INTEGER, Row AS INTEGER, CanSelect AS INTEGER)
  Log = Log + "cell" + STR$(Col) + STR$(Row) + " "
  IF Col = 3 THEN CanSelect = False
END SUB

SUB Measure (Index AS INTEGER, Height AS INTEGER)
  Height = 10 + Index * 10
END SUB

SUB DrawItem (Index AS INTEGER, State AS INTEGER, R AS QRECT)
  IF Index = 0 THEN Lbl2.Caption = ""
  Lbl2.Caption = Lbl2.Caption + STR$(R.Top) + "-" + STR$(R.Bottom) + " "
END SUB

Form.ShowModal

' QLISTBOX Columns (items flow down each column, then into the next) and an
' owner-drawn QCOMBOBOX (Style = csOwnerDrawVariable: OnMeasureItem's
' heights, OnDrawItem draws each item; the box shows the picked one).
$INCLUDE "RAPIDQ.INC"
DECLARE SUB Picked
DECLARE SUB ComboMeasure (Index AS INTEGER, Height AS INTEGER)
DECLARE SUB ComboDraw (Index AS INTEGER, State AS INTEGER, R AS QRECT)
DECLARE SUB ComboChange
DIM Rects AS STRING
CREATE Form AS QFORM
  Caption = "list columns"
  Width = 360
  Height = 260
  CREATE Lst AS QLISTBOX
    Left = 5
    Top = 5
    Width = 204
    Height = 84
    ItemHeight = 20
    Columns = 2
    AddItems "a0", "a1", "a2", "a3", "a4", "a5", "a6"
    OnClick = Picked
  END CREATE
  CREATE Cb AS QCOMBOBOX
    Left = 220
    Top = 5
    Width = 120
    Height = 24
    Style = csOwnerDrawVariable
    OnMeasureItem = ComboMeasure
    OnDrawItem = ComboDraw
    OnChange = ComboChange
    AddItems "red", "green", "blue"
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 5
    Top = 120
    Width = 340
    Caption = "-"
  END CREATE
  CREATE Lbl2 AS QLABEL
    Left = 5
    Top = 145
    Width = 340
    Caption = "-"
  END CREATE
END CREATE
SUB Picked
  Lbl.Caption = "i" + STR$(Lst.ItemIndex) + " " + Lst.Item(Lst.ItemIndex) + " cols" + STR$(Lst.Columns)
END SUB
SUB ComboMeasure (Index AS INTEGER, Height AS INTEGER)
  Height = 18 + Index * 2
END SUB
SUB ComboDraw (Index AS INTEGER, State AS INTEGER, R AS QRECT)
  IF Index = 0 THEN Rects = ""
  Cb.FillRect(R.Left, R.Top, R.Right, R.Bottom, &HFF SHL (Index * 8))
  Cb.TextOut(R.Left + 4, R.Top + 2, Cb.Item(Index), &HFFFFFF, -1)
  Rects = Rects + STR$(R.Top) + "-" + STR$(R.Bottom) + "/" + STR$(R.Right) + " "
END SUB
SUB ComboChange
  Lbl2.Caption = "c" + STR$(Cb.ItemIndex) + " " + Rects
END SUB
Form.ShowModal

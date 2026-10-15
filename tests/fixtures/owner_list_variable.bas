' A lbOwnerDrawVariable QLISTBOX on a form nobody touches (RapidQ's
' examples/listbox/owndraw): OnMeasureItem gives each item its height, then
' OnDrawItem draws every item, at once — not when the mouse first moves
' (the interpreter waited for the system's next event before running the
' drawing handlers its measuring had made due).
$INCLUDE "RAPIDQ.INC"
DECLARE SUB Measure (Index AS INTEGER, Height AS INTEGER)
DECLARE SUB DrawItem (Index AS INTEGER, State AS BYTE, Rect AS QRECT)
DIM Log AS STRING
CREATE Form AS QFORM
    Caption = "owner list, variable"
    Width = 260
    Height = 200
    CREATE Lst AS QLISTBOX
        Left = 0
        Top = 0
        Width = 200
        Height = 120
        Style = lbOwnerDrawVariable
        OnMeasureItem = Measure
        OnDrawItem = DrawItem
        AddItems "Alpha", "Beta", "Gamma"
        ItemIndex = 0
    END CREATE
    CREATE Lbl AS QLABEL
        Left = 0
        Top = 130
        Width = 250
        Caption = "-"
    END CREATE
END CREATE
SUB Measure (Index AS INTEGER, Height AS INTEGER)
    Height = 20 + Index * 10
END SUB
SUB DrawItem (Index AS INTEGER, State AS BYTE, Rect AS QRECT)
    IF Index = 0 THEN Log = ""
    IF State = 0 THEN
        Lst.FillRect(Rect.Left, Rect.Top, Rect.Right, Rect.Bottom, &H00FF00)
    ELSE
        Lst.FillRect(Rect.Left, Rect.Top, Rect.Right, Rect.Bottom, &HFF0000)
    END IF
    Lst.TextOut(24, Rect.Top + 4, Lst.Item(Index), 0, -1)
    Log = Log + STR$(Index) + ":" + STR$(Rect.Top) + "-" + STR$(Rect.Bottom) + " "
    Lbl.Caption = Log
END SUB
Form.ShowModal

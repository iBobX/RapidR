' An owner-drawn QLISTBOX (Style = lbOwnerDrawFixed): OnDrawItem(Index, State,
' Rect) for every item after a change, State 0 for the selected item; what
' the handler draws (FillRect, TextOut, Draw) is the item.
$INCLUDE "RAPIDQ.INC"
DECLARE SUB DrawItem (Index AS INTEGER, State AS BYTE, Rect AS QRECT)
DECLARE SUB Next1
DECLARE SUB Clicked
DIM Log AS STRING
DIM Rounds AS INTEGER
DIM Chip AS QBITMAP
Chip.Width = 10
Chip.Height = 10
Chip.FillRect(0, 0, 10, 10, &H0000FF)
CREATE Form AS QFORM
    Caption = "owner list"
    Width = 300
    Height = 260
    CREATE Lst AS QLISTBOX
        Left = 10
        Top = 10
        Width = 200
        Height = 120
        Style = lbOwnerDrawFixed
        ItemHeight = 24
        OnDrawItem = DrawItem
        OnClick = Clicked
        AddItems "Alpha", "Beta", "Gamma"
        ItemIndex = 1
    END CREATE
    CREATE Btn AS QBUTTON
        Left = 10
        Top = 140
        Caption = "Next"
        OnClick = Next1
    END CREATE
    CREATE Lbl AS QLABEL
        Left = 10
        Top = 180
        Caption = "-"
    END CREATE
END CREATE
SUB DrawItem (Index AS INTEGER, State AS BYTE, Rect AS QRECT)
    IF Index = 0 THEN Log = "": Rounds = Rounds + 1
    IF State = 0 THEN
        Lst.FillRect(Rect.Left, Rect.Top, Rect.Right, Rect.Bottom, &H00FF00)
    ELSE
        Lst.FillRect(Rect.Left, Rect.Top, Rect.Right, Rect.Bottom, &HFFFFFF)
    END IF
    Lst.TextOut(24, Rect.Top + 4, Lst.Item(Index), 0, -1)
    Lst.Draw(4, Rect.Top + 4, Chip.BMP)
    Log = Log + STR$(Index) + ":" + STR$(State) + ";"
    IF Index = 2 THEN Lbl.Caption = "r" + STR$(Rounds) + " " + Log + " " + STR$(Rect.Left) + "," + STR$(Rect.Top) + "," + STR$(Rect.Right) + "," + STR$(Rect.Bottom) + " h" + STR$(Lst.ItemHeight)
END SUB
SUB Next1
    Lst.ItemIndex = 2
END SUB
SUB Clicked
END SUB
Form.ShowModal

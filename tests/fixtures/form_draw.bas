' Drawing on a QFORM itself, in its OnPaint: the surface lies under the
' controls; Pixel and TextWidth read back; a new size paints again.
DECLARE SUB FormPaint
DECLARE SUB Grow
DIM paints AS INTEGER
CREATE Win AS QFORM
    Caption = "form draw"
    Width = 300
    Height = 200
    Color = &HE0E0E0
    OnPaint = FormPaint
    CREATE big AS QBUTTON
        Left = 10
        Top = 100
        Caption = "Grow"
        OnClick = Grow
    END CREATE
    CREATE lbl AS QLABEL
        Left = 10
        Top = 140
        Caption = "-"
    END CREATE
END CREATE
SUB FormPaint
    paints = paints + 1
    Win.FillRect(10, 10, 60, 40, &HFF)
    Win.Font.Size = 12
    Win.TextOut(80, 10, "Hello", 0, -1)
    lbl.Caption = "paints" + STR$(paints) + "|" + STR$(Win.Pixel(20, 20)) + "|" + STR$(Win.Pixel(200, 100)) + "|" + STR$(Win.TextWidth("Hello")) + "|" + STR$(Win.ClientWidth)
END SUB
SUB Grow
    Win.Width = 340
END SUB
Win.ShowModal

' QCANVAS: OnPaint fires when the form is built and on Repaint (and after a
' resize); what a handler draws stays; pixels and text metrics read back.
DECLARE SUB PaintIt
DECLARE SUB FormPaint
DECLARE SUB Again
DECLARE SUB Grow
DIM paints AS INTEGER
DIM fpaints AS INTEGER
CREATE Form AS QFORM
    Caption = "canvas onpaint"
    Width = 260
    Height = 200
    OnPaint = FormPaint
    CREATE btn AS QBUTTON
        Left = 10
        Top = 100
        Caption = "Again"
        OnClick = Again
    END CREATE
    CREATE big AS QBUTTON
        Left = 90
        Top = 100
        Caption = "Grow"
        OnClick = Grow
    END CREATE
    CREATE lbl AS QLABEL
        Left = 10
        Top = 130
        Caption = "-"
    END CREATE
    CREATE C AS QCANVAS
        Left = 10
        Top = 10
        Width = 200
        Height = 80
        Color = &H00FF00
        OnPaint = PaintIt
    END CREATE
END CREATE
SUB PaintIt
    paints = paints + 1
    C.FillRect(0, 0, 20, 20, &HFF)
    C.Font.Size = 12
    C.TextOut(30, 5, "n" + STR$(paints), 0, -1)
    lbl.Caption = "paints" + STR$(paints) + "|form" + STR$(fpaints) + "|" + STR$(C.Pixel(5, 5)) + "|" + STR$(C.Pixel(100, 60)) + "|" + STR$(C.Width) + "x" + STR$(C.Height) + "|" + STR$(C.TextWidth("Hello"))
END SUB
SUB Again
    C.Repaint
END SUB
SUB Grow
    C.Width = 220
END SUB
SUB FormPaint
    fpaints = fpaints + 1
END SUB
Form.ShowModal

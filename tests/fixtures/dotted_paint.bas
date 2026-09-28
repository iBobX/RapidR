' A canvas whose OnPaint handler is a SUB with a dotted name
' (`bups.OnPaint = bups.paint`, `SUB bups.paint`, as in the corpus's Sokoban).
CREATE Form AS QFORM
    Caption = "dotted paint"
    Width = 240
    Height = 180
    CREATE Lbl AS QLABEL
        Top = 120
        Width = 200
    END CREATE
END CREATE
DIM bups AS QCANVAS
bups.Width = 100
bups.Height = 60
bups.Parent = Form
bups.OnPaint = bups.paint
SUB bups.paint
    bups.FillRect(0, 0, 100, 60, &H0000FF)
    Lbl.Caption = "painted " + STR$(bups.Pixel(5, 5))
END SUB
Form.ShowModal

' A form is drawn on like a canvas (the surface lies under its controls).
CREATE Win AS QFORM
    Width = 300
    Height = 200
    Color = &H00FF00
    CREATE Btn AS QBUTTON
        Left = 10
        Top = 10
        Caption = "b"
    END CREATE
END CREATE
PRINT "bg "; Win.Pixel(150, 100)
Win.FillRect(10, 20, 30, 40, &HFF)
PRINT "fill "; Win.Pixel(10, 20); " "; Win.Pixel(29, 39); " "; Win.Pixel(30, 40)
Win.Line(0, 60, 49, 60, &HFF0000)
PRINT "line "; Win.Pixel(0, 60); " "; Win.Pixel(49, 60); " "; Win.Pixel(50, 60)
Win.Font.Name = "Arial"
Win.Font.Size = 12
PRINT "textwidth "; Win.TextWidth("Hello")
Win.TextOut(60, 20, "Hi", &H0, -1)
n = 0
FOR y = 20 TO 45
    FOR x = 60 TO 90
        IF Win.Pixel(x, y) = 0 THEN n = n + 1
    NEXT x
NEXT y
PRINT "ink "; n
PRINT "caption "; Btn.Caption
Win.Cls
PRINT "cls "; Win.Pixel(10, 20)

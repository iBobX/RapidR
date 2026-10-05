' A QCANVAS's pixels through the shared bitmap model. Where nothing is drawn it
' shows its parent's colour (here the form's clBtnFace, F0F0F0), whatever its
' own Color — RapidQ's QCANVAS is a TPaintBox (RC.EXE: docs/rapidq-ground-truth.md).
CREATE Form AS QFORM
    Width = 300
    Height = 200
    CREATE Canvas1 AS QCANVAS
        Left = 10
        Top = 10
        Width = 100
        Height = 60
        Color = &H00FF00
    END CREATE
END CREATE

PRINT "bg "; Canvas1.Pixel(5, 5)
Canvas1.FillRect(10, 10, 20, 20, &HFF)
PRINT "fill "; Canvas1.Pixel(10, 10); " "; Canvas1.Pixel(19, 19); " "; Canvas1.Pixel(20, 20)
Canvas1.Line(0, 0, 9, 0, &HFF0000)
PRINT "line "; Canvas1.Pixel(0, 0); " "; Canvas1.Pixel(9, 0); " "; Canvas1.Pixel(10, 0)
Canvas1.Rectangle(30, 10, 40, 20, &H123456)
PRINT "rect "; Canvas1.Pixel(30, 10); " "; Canvas1.Pixel(39, 19); " "; Canvas1.Pixel(35, 15)
Canvas1.PenColor = &H778899
Canvas1.Line(0, 30, 9, 30)
PRINT "pen "; Canvas1.Pixel(3, 30)
Canvas1.Circle(60, 40, 5, &H0000FF)
PRINT "circle "; Canvas1.Pixel(60, 35); " "; Canvas1.Pixel(60, 40)
Canvas1.Font.Name = "Arial"
Canvas1.Font.Size = 12
PRINT "textwidth "; Canvas1.TextWidth("Hello")
PRINT "textheight "; Canvas1.TextHeight("Hello")
Canvas1.TextOut(2, 40, "Hi", &H0, -1)
n = 0
FOR y = 40 TO 60
    FOR x = 2 TO 30
        IF Canvas1.Pixel(x, y) = 0 THEN n = n + 1
    NEXT x
NEXT y
PRINT "ink "; n
Canvas1.Cls
PRINT "cls "; Canvas1.Pixel(10, 10)
PRINT "size "; Canvas1.Width; " "; Canvas1.Height
Canvas1.Width = 120
Canvas1.FillRect(100, 0, 110, 10, 7)
PRINT "grown "; Canvas1.Pixel(105, 5); " "; Canvas1.Pixel(115, 5)

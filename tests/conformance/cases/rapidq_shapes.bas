' Rectangle, RoundRect, Circle, FillRect and Line on a QBITMAP as RapidQ
' draws them (RC.EXE, 2026-10-22, docs/rapidq-ground-truth.md; the .expected
' is RC.EXE's output): Rectangle is an outline in its colour, its inside left
' as it was (no fill, whatever was drawn before); RoundRect is filled in its
' colour; Circle(x1, y1, x2, y2, c, fill) is outlined in c and filled with
' fill. The right and bottom edges (x2, y2) are left out of all four; a Line
' includes its end. ("." the yellow background, "o" red, "f" green, "b" the
' blue a FillRect drew last.) Only pixels off an ellipse's curve are read:
' the curve's own steps are Windows' drawing, not RapidQ's.
DIM B AS QBITMAP
B.Width = 16
B.Height = 12

FUNCTION Px (x AS INTEGER, y AS INTEGER) AS STRING
  DIM p AS LONG
  p = B.Pixel(x, y)
  IF p = &H00FFFF THEN
    Px = "."
  ELSEIF p = &H0000FF THEN
    Px = "o"
  ELSEIF p = &H00FF00 THEN
    Px = "f"
  ELSEIF p = &HFF0000 THEN
    Px = "b"
  ELSE
    Px = "?"
  END IF
END FUNCTION

SUB Fresh
  B.FillRect 0, 0, 16, 12, &H00FFFF
  B.FillRect 15, 11, 16, 12, &HFF0000
END SUB

SUB Dump (Title AS STRING)
  DIM x AS INTEGER
  DIM y AS INTEGER
  DIM s AS STRING
  PRINT Title
  FOR y = 0 TO 11
    s = ""
    FOR x = 0 TO 15
      s = s + Px(x, y)
    NEXT
    PRINT s
  NEXT
END SUB

Fresh
B.FillRect 2, 2, 6, 5, &H0000FF
Dump "FillRect 2,2,6,5"
Fresh
B.Rectangle 2, 2, 10, 8, &H0000FF
Dump "Rectangle 2,2,10,8"
Fresh
B.Rectangle 10, 8, 2, 2, &H0000FF
Dump "Rectangle 10,8,2,2"
Fresh
B.Line 2, 2, 10, 2, &H0000FF
Dump "Line 2,2,10,2"

Fresh
B.Circle 0, 0, 4, 4, &H0000FF, &H00FF00
B.Rectangle 6, 2, 14, 8, &H0000FF
PRINT "Rectangle after Circle: "; Px(6, 2); Px(10, 5); Px(13, 7); Px(14, 5)

Fresh
B.Circle 2, 2, 14, 10, &H0000FF, &H00FF00
PRINT "Circle: "; Px(7, 6); Px(2, 6); Px(13, 6); Px(14, 6); Px(7, 10); Px(2, 2)

Fresh
B.Circle 10, 8, 2, 2, &H0000FF, &H00FF00
PRINT "Circle reversed: "; Px(5, 5); Px(2, 5); Px(10, 5); Px(9, 5)

Fresh
B.Circle 3, 3, 9, 9, &H0000FF, &H0000FF
PRINT "Circle one colour: "; Px(5, 5); Px(3, 6)

Fresh
B.RoundRect 2, 2, 14, 10, 6, 6, &H0000FF
PRINT "RoundRect: "; Px(7, 6); Px(2, 6); Px(13, 6); Px(14, 6); Px(7, 9); Px(7, 10); Px(2, 2)

Fresh
B.Circle 0, 0, 4, 4, &H0000FF, &H00FF00
B.RoundRect 6, 2, 14, 10, 4, 4, &H0000FF
PRINT "RoundRect after Circle: "; Px(10, 6); Px(14, 6)

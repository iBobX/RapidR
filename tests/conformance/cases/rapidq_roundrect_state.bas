' RoundRect's colour is the whole of it (RC.EXE, 2026-11-06,
' docs/rapidq-ground-truth.md; the .expected is RC.EXE's output): the
' rounded box is filled with the colour argument and nothing else — no
' separate outline, and what an earlier statement drew with (a Circle's
' outline and fill, a Line's pen, a Rectangle's pen) never leaks into it.
' Its right and bottom edges (x2, y2) are left out, as the other shapes'.
' ("." the yellow background, "o" red, "g" green, "w" white, "b" blue.)
' The same on a QCANVAS and a QFORM's own surface read the same pixels
' (the w_roundrect probe: tests/conformance/.work/rq-roundrect).
DIM B AS QBITMAP
B.Width = 40
B.Height = 24

FUNCTION Px (x AS INTEGER, y AS INTEGER) AS STRING
  DIM p AS LONG
  p = B.Pixel(x, y)
  IF p = &H00FFFF THEN
    Px = "."
  ELSEIF p = &H0000FF THEN
    Px = "o"
  ELSEIF p = &H00FF00 THEN
    Px = "g"
  ELSEIF p = &HFFFFFF THEN
    Px = "w"
  ELSEIF p = &HFF0000 THEN
    Px = "b"
  ELSE
    Px = "?"
  END IF
END FUNCTION

SUB Fresh
  B.FillRect 0, 0, 40, 24, &H00FFFF
END SUB

' The box 4,4 .. 24,16 and what a few of its pixels show: the middle, the
' left and right edges (inside and just out), the top and bottom, a corner
' and a point just inside the corner.
SUB Look (Title AS STRING)
  PRINT Title; ": "; Px(14, 10); Px(4, 10); Px(5, 10); Px(23, 10); Px(24, 10); Px(14, 4); Px(14, 15); Px(14, 16); Px(4, 4); Px(7, 7)
END SUB

Fresh
B.RoundRect 4, 4, 24, 16, 8, 8, &H0000FF
Look "alone"

Fresh
B.Circle 28, 4, 38, 14, &H00FF00, &HFFFFFF
B.RoundRect 4, 4, 24, 16, 8, 8, &HFF0000
Look "after a Circle (green outline, white fill)"

Fresh
B.Line 28, 20, 38, 20, &H00FF00
B.RoundRect 4, 4, 24, 16, 8, 8, &HFF0000
Look "after a Line"

Fresh
B.Rectangle 28, 4, 38, 14, &H00FF00
B.RoundRect 4, 4, 24, 16, 8, 8, &HFF0000
Look "after a Rectangle"

Fresh
B.RoundRect 24, 16, 4, 4, 8, 8, &H0000FF
Look "corners reversed"

' TextRect(Rect, x, y, S$, fc, bc): the text TextOut draws at (x, y), only
' inside Rect (its right and bottom edges left out). A background colour
' fills all of Rect first (Windows' ExtTextOut, as Delphi's TextRect with a
' solid brush); -1 leaves what's there. RapidQ's own TextRect stops the
' program (RC.EXE, RapidQ 2006: an access violation on a QBITMAP, a list
' index error on a QCANVAS — every call), so this is RapidR's behaviour.
DIM B AS QBITMAP
DIM R AS QRECT
B.Width = 100: B.Height = 50

SUB Count (Col AS LONG, Label AS STRING)
  DIM X AS INTEGER, Y AS INTEGER, N AS LONG
  DIM X1 AS INTEGER, Y1 AS INTEGER, X2 AS INTEGER, Y2 AS INTEGER
  X1 = 999: Y1 = 999: X2 = -1: Y2 = -1: N = 0
  FOR Y = 0 TO 49
    FOR X = 0 TO 99
      IF B.Pixel(X, Y) = Col THEN
        N = N + 1
        IF X < X1 THEN X1 = X
        IF Y < Y1 THEN Y1 = Y
        IF X > X2 THEN X2 = X
        IF Y > Y2 THEN Y2 = Y
      END IF
    NEXT
  NEXT
  PRINT Label; " n="; N; " box="; X1; ","; Y1; " "; X2; ","; Y2
END SUB

R.Left = 10: R.Top = 10: R.Right = 60: R.Bottom = 30
' A background: Rect filled, the text clipped to it.
B.FillRect(0, 0, 100, 50, &HFFFFFF)
B.TextRect(R, 12, 12, "WWWWWWWWWWWW", 0, &HFF)
Count &HFF, "red"
Count &HFFFFFF, "white"
' -1: transparent.
B.FillRect(0, 0, 100, 50, &HFFFFFF)
B.TextRect(R, 12, 12, "WWWWWWWWWWWW", 0, -1)
Count &HFFFFFF, "white"
PRINT HEX$(B.Pixel(55, 28)); " "; HEX$(B.Pixel(61, 14))
' Clipped TextRect = TextOut inside the rectangle.
B.FillRect(0, 0, 100, 50, &HFFFFFF)
B.TextOut(12, 12, "W", 0, -1)
DIM Ink AS LONG, X AS INTEGER, Y AS INTEGER
Ink = 0
FOR Y = 0 TO 49: FOR X = 0 TO 99: Ink = Ink + (B.Pixel(X, Y) AND 255): NEXT: NEXT
B.FillRect(0, 0, 100, 50, &HFFFFFF)
B.TextRect(R, 12, 12, "W", 0, -1)
FOR Y = 0 TO 49: FOR X = 0 TO 99: Ink = Ink - (B.Pixel(X, Y) AND 255): NEXT: NEXT
PRINT "same as TextOut: "; Ink = 0
' Text starting outside the rectangle: only its part inside shows.
B.FillRect(0, 0, 100, 50, &HFFFFFF)
B.TextRect(R, 0, 0, "WWWWWWWWWWWW", 0, &HFF00)
Count &HFF00, "green"
' Text wholly outside: just the fill.
B.FillRect(0, 0, 100, 50, &HFFFFFF)
B.TextRect(R, 70, 12, "W", 0, &HFF0000)
Count &HFF0000, "blue"
Count 0, "black"
' A rectangle given right to left / bottom to top is the same rectangle.
R.Left = 60: R.Top = 30: R.Right = 10: R.Bottom = 10
B.FillRect(0, 0, 100, 50, &HFFFFFF)
B.TextRect(R, 70, 12, "W", 0, &HFF0000)
Count &HFF0000, "blue"
' On a canvas, in its font.
CREATE Form AS QFORM
  CREATE C AS QCANVAS
    Width = 80: Height = 40
  END CREATE
END CREATE
R.Left = 5: R.Top = 5: R.Right = 25: R.Bottom = 15
C.TextRect(R, 6, 6, "Hi", &HFFFFFF, &H800000)
PRINT HEX$(C.Pixel(24, 14)); " "; HEX$(C.Pixel(25, 15)); " "; HEX$(C.Pixel(4, 4))

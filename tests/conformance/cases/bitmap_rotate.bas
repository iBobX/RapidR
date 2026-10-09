' QBITMAP.Rotate(xOrigin, yOrigin, Angle), as RC.EXE (RapidQ 2006, in the
' Windows VM) runs it: Angle in degrees, anticlockwise on the screen; the
' bitmap keeps its size; each pixel takes the one that turns onto it (the
' nearest), and where that is outside the bitmap, the colour pixel (0, 0)
' had before the turn. The expected output is RC.EXE's.
DIM B AS QBITMAP
DIM X AS INTEGER, Y AS INTEGER, C AS LONG, S AS STRING

' Each pixel's colour names it: x + y * 256 + &H10000.
SUB Setup
  B.Width = 12: B.Height = 10
  FOR Y = 0 TO 9
    FOR X = 0 TO 11
      B.Pset(X, Y, X + Y * 256 + &H10000)
    NEXT
  NEXT
END SUB

' Where each pixel came from.
SUB Dump (Label AS STRING)
  PRINT "-- "; Label
  FOR Y = 0 TO 9
    S = ""
    FOR X = 0 TO 11
      C = B.Pixel(X, Y)
      IF C >= &H10000 AND C < &H20000 THEN
        S = S + STR$((C AND 255)) + "," + LTRIM$(STR$((C \ 256) AND 255)) + " "
      ELSE
        S = S + " " + HEX$(C) + " "
      END IF
    NEXT
    PRINT S
  NEXT
END SUB

Setup
B.Rotate(3, 4, 135)
Dump "3,4 135"
Setup
B.Rotate(5, 5, 200)
Dump "5,5 200"
Setup
B.Rotate(0, 0, 90)
Dump "0,0 90"
Setup
B.Rotate(6, 4, 370)
Dump "6,4 370"
Setup
B.Rotate(6, 4, 0)
Dump "6,4 0"
Setup
B.Rotate(-20, 4, 10)
Dump "-20,4 10"
Setup
B.Rotate(6, 5, -45)
Dump "6,5 -45"
Setup
B.Rotate(4, 4, 64)
Dump "4,4 64"

' A line turned a quarter: it points up (anticlockwise).
B.Width = 21: B.Height = 21
B.FillRect(0, 0, 21, 21, &HFF0000)
B.Line(10, 10, 18, 10, &HFF)
B.Rotate(10, 10, 90)
PRINT HEX$(B.Pixel(10, 5)); " "; HEX$(B.Pixel(15, 10)); " "; HEX$(B.Pixel(10, 15)); " "; HEX$(B.Pixel(5, 10))

' What fills where nothing turns onto: pixel (0, 0)'s colour from before.
B.Width = 12: B.Height = 10
B.FillRect(0, 0, 12, 10, &HFF0000)
B.Pset(0, 0, &H123456)
B.Rotate(0, 0, 90)
PRINT HEX$(B.Pixel(0, 0)); " "; HEX$(B.Pixel(5, 5)); " "; HEX$(B.Pixel(11, 9)); " "; HEX$(B.Pixel(3, 0))
B.FillRect(0, 0, 12, 10, &HFF0000)
B.Pset(0, 0, &H123456)
B.Rotate(6, 5, 180)
PRINT HEX$(B.Pixel(0, 0)); " "; HEX$(B.Pixel(5, 5)); " "; HEX$(B.Pixel(11, 9)); " "; HEX$(B.Pixel(11, 0))
PRINT B.Width; B.Height
B.Width = 1: B.Height = 1
B.Pset(0, 0, &H777777)
B.Rotate(0, 0, 45)
PRINT HEX$(B.Pixel(0, 0))
B.Width = 0
B.Rotate(0, 0, 45)
PRINT "empty ok"

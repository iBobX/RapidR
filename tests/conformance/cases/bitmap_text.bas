' Text on a QBITMAP from RapidR's built-in fonts (Liberation, with Arial's,
' Times New Roman's and Courier New's widths): TextWidth, TextHeight, and
' TextOut's pixels, the same in every build.
DIM F AS QFONT
DIM B AS QBITMAP
B.Width = 80: B.Height = 24
F.Name = "Arial": F.Size = 12
B.Font = F
PRINT B.TextWidth("Hello"); " "; B.TextHeight("Hello")
F.Name = "Courier New"
B.Font = F
PRINT B.TextWidth("iiii") = B.TextWidth("MMMM"); " "; B.TextWidth("iiii")
F.Name = "Times New Roman": F.Bold = 1
B.Font = F
PRINT B.TextWidth("Times")
F.Name = "Arial": F.Bold = 0
B.Font = F
B.FillRect(0, 0, 80, 24, &HFFFFFF)
B.TextOut(2, 2, "Hi!", 0, -1)
DIM Dark AS INTEGER, Grey AS INTEGER
FOR Y = 0 TO 23
  FOR X = 0 TO 79
    P = B.Pixel(X, Y)
    IF P = 0 THEN
      Dark = Dark + 1
    ELSEIF P <> &HFFFFFF THEN
      Grey = Grey + 1
    END IF
  NEXT
NEXT
PRINT Dark; " "; Grey
B.TextOut(40, 2, "x", &HFF, &H00FF00)
PRINT HEX$(B.Pixel(40, 2))

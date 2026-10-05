' QRECT's fields as RapidQ's compiler (RC.EXE) stores them: 0 until set,
' 32-bit integers cut toward zero, a string 0, out of range -2147483648;
' CREATE works, a SUB gets it by reference, SIZEOF is 16 (all checked
' against RC.EXE: docs/rapidq-ground-truth.md).
$APPTYPE CONSOLE
SUB Widen (R AS QRECT)
  PRINT R.Left; " "; R.Right
  R.Right = R.Right + 10
END SUB
DIM r AS QRECT
PRINT r.Left; " "; r.Top; " "; r.Right; " "; r.Bottom
r.Left = 5: r.Top = 6: r.Right = 20: r.Bottom = 30
PRINT r.Left; " "; r.Top; " "; r.Right; " "; r.Bottom
r.Left = 3.7
PRINT r.Left
r.Top = -2.5
PRINT r.Top
r.Bottom = -3.9
PRINT r.Bottom
r.Left = "12"
PRINT r.Left
r.Top = 2147483648
PRINT r.Top
r.Right = 4294967297
PRINT r.Right
r.Left = 1: r.Right = 2
Widen r
PRINT r.Right
PRINT SIZEOF(r)
WITH r
  .Left = 4
  .Right = .Left + 6
END WITH
PRINT r.Right
CREATE c AS QRECT
  Left = 3
  Bottom = 9
END CREATE
PRINT c.Left; " "; c.Bottom

' Still more forms RapidQ's example programs use that RC.EXE takes (the
' .expected is RC.EXE's output): a call in a WITH whose arguments start
' with the WITH's members after a space (`MemCpy .TB, .Button(i), n`,
' QToolBar.inc), a `_` alone inside a line (an operand of its own, 0:
' `t _ (3 + 1)` is t — games/asteroids.bas), and READ into a method
' (`Read L.AddItems N` adds the DATA items — music/StutterBox.rqb).
$APPTYPE CONSOLE
SUB PR (x AS INTEGER, y AS INTEGER)
  PRINT x; "/"; y
END SUB
TYPE TT EXTENDS QOBJECT
  A AS INTEGER
  B AS INTEGER
  WITH This
  SUB Go
    .A = 5
    .B = 6
    PR .A, .B
    PR .B,.A
  END SUB
  END WITH
END TYPE
DIM o AS TT
o.Go
WITH o
  PR .A, 1
END WITH
DIM a AS INTEGER, t AS INTEGER
t = 7
a = t _ (3 + 1)
PRINT a
a = t _ 2
PRINT a
DIM L AS QSTRINGLIST
DIM N AS INTEGER
FOR N = 0 TO 1
  READ L.AddItems N
NEXT N
PRINT L.ItemCount
PRINT L.Item(0); "|"; L.Item(1); "|"
DATA "x", "y", "z"

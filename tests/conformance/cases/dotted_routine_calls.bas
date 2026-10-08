' Routines defined with a dotted name (`SUB Draw.Box`, RapidQ's
' console/3dbox example) are called by that name — a statement, with or
' without parentheses, and a FUNCTION in an expression — on both
' backends, as RC.EXE does (checked: box12 / 7 / 22 / box78)
$APPTYPE CONSOLE
SUB Draw.Box (X AS SHORT, Y AS SHORT)
  PRINT "box"; X; Y
END SUB
FUNCTION Calc.Sum (a AS LONG, b AS LONG) AS LONG
  Result = a + b
END FUNCTION
Draw.Box 1, 2
PRINT Calc.Sum(3, 4)
x = Calc.Sum(5, 6) * 2
PRINT x
Draw.Box(7, 8)

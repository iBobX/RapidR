' INT, FIX, ROUND, CINT, CLNG, CEIL, FLOOR and FRAC as RapidQ's own compiler
' (RC.EXE) has them — the .expected is its output (docs/rapidq-ground-truth.md):
' INT truncates toward zero like FIX (the manual says "largest integer less
' than or equal", RapidQ doesn't), as a float's whole number; ROUND, CINT and
' CLNG are INT(x + 0.5) (2.5 → 3, -2.5 → -2, -2.7 → -2, -3.99 → -3), CEIL
' and FLOOR round as their names say — all four 32-bit integers (beyond:
' -2147483648, so ROUND(1E10) / 1E10 is -0.214748365).
PRINT "int  "; INT(2.5); " "; INT(2.7); " "; INT(-2.5); " "; INT(-2.7); " "; INT(-0.5); " "; INT(0.5); " "; INT(-984.147)
PRINT "fix  "; FIX(2.7); " "; FIX(-2.7); " "; FIX(-0.5)
PRINT "rnd  "; ROUND(2.5); " "; ROUND(3.5); " "; ROUND(0.5); " "; ROUND(1.5); " "; ROUND(2.4999); " "; ROUND(2.5000001)
PRINT "rnd- "; ROUND(-0.5); " "; ROUND(-1.5); " "; ROUND(-2.5); " "; ROUND(-2.2); " "; ROUND(-2.7); " "; ROUND(-3.99)
PRINT "cint "; CINT(2.5); " "; CINT(3.5); " "; CINT(-2.5); " "; CINT(-2.7); " "; CINT(-0.5); " "; CINT(0.5); " "; CINT(1.5)
PRINT "clng "; CLNG(2.5); " "; CLNG(-3.5); " "; CLNG(-2.7)
PRINT "ceil "; CEIL(1.5); " "; CEIL(-1.5); " "; CEIL(2.000001); " "; CEIL(2)
PRINT "flr  "; FLOOR(1.5); " "; FLOOR(-1.5); " "; FLOOR(-0.5); " "; FLOOR(2)
PRINT "frac "; FRAC(2.75); " "; FRAC(-2.75); " "; FRAC(1E10 + 0.5)
PRINT "big  "; INT(1E10) / 1E10; " "; FIX(1E10) / 1E10; " "; ROUND(1E10) / 1E10; " "; CINT(1E10) / 1E10
PRINT "big2 "; CEIL(1E10) / 1E10; " "; FLOOR(-1E10) / 1E10; " "; CLNG(1E10) / 1E10
PRINT "half "; INT(2.5) / 4; " "; CEIL(2.5) / 4; " "; FLOOR(2.5) / 4; " "; ROUND(2.5) / 4
DIM d AS DOUBLE
d = -9.84147
PRINT "var  "; INT(d * 100); " "; ROUND(d * 100); " "; FIX(d); " "; CINT(d)

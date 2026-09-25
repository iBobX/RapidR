' RapidQ DIM/DEFxxx forms (manual: DIM, DEF...): per-name AS, untyped names are VARIANT,
' (a, b)(n) groups, = value and = {...} initializers filled in memory order, typed defaults.
DIM a AS INTEGER = 5, s AS STRING = "hi", v
DIM (x, y, z) AS DOUBLE
DIM (p, q)(2) AS INTEGER
DEFSTR s1, s2 = "two", s3
DEFINT n = 99, arr(1 TO 4) = {10, 20, _
   30, 40}, k
DEFLNG grid(1, 2) = {1, 2, 3, 4, 5, 6}
DIM b, c AS LONG
PRINT a; " "; s; " ["; v; "] "; x + y + z; " "; UBOUND(p); UBOUND(q)
PRINT "["; s1; "] "; s2; " ["; s3; "] "; n; " "; k
PRINT arr(1); arr(2); arr(3); arr(4); " "; LBOUND(arr)
PRINT grid(0,0); grid(0,1); grid(0,2); grid(1,0); grid(1,1); grid(1,2)
PRINT "["; b; "] ["; c; "]"
SUB Local
  DEFDBL r = 2.5
  DIM t AS STRING = "local"
  PRINT r; " "; t
END SUB
Local
IF a = 5 THEN DEFINT w = 7: PRINT w

' Dividing by zero as RapidQ does it (RC.EXE, docs/rapidq-ground-truth.md):
' `/` gives an infinity (0 / 0 a NaN) — whole numbers beyond 32 bits, so
' PRINT shows -2147483648 — while `\` and MOD stop the program with
' "Division by zero" (RapidQ's EDivByZero), here after "before 3 ".
DIM j AS INTEGER
DIM d AS DOUBLE
PRINT 7 / j; " "; -7 / d; " "; 0 / d
d = 7 / j
PRINT d > 1E300; " "; STR$(d)
PRINT "before "; 7 \ 2; " "; 7 \ j
PRINT "never"

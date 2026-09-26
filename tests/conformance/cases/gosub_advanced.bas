' Nested GOSUB, GOSUB inside a SUB, a backwards GOTO loop, line numbers, and `DoEvents:` as a call.
DIM n AS INTEGER
n = 0
Again:
n = n + 1
IF n < 3 THEN GOTO Again
PRINT "loop:" + STR$(n)
GOSUB Outer
PRINT "back in main"
Worker
DoEvents: PRINT "doevents ok"
GOTO 200
100 PRINT "skipped"
200 PRINT "line numbers ok"
END

Outer:
  PRINT "outer"
  GOSUB Inner
  PRINT "outer again"
RETURN

Inner:
  PRINT "inner"
RETURN

SUB Worker()
  GOSUB Local
  PRINT "sub done"
  EXIT SUB
Local:
  PRINT "sub gosub"
  RETURN
END SUB

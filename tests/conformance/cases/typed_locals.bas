' Typed locals: native builds keep them as Rust numbers (rapidr-codegen-rust
' `typed`); the interpreter as Values. Both must print exactly the same.
SUB Arith
  DIM a AS LONG, b AS LONG, d AS DOUBLE, e AS DOUBLE, s AS SHORT, by AS BYTE
  a = 7: b = 2
  PRINT a + b; " "; a - b; " "; a * b; " "; a / b; " "; a \ b; " "; a MOD b; " "; a ^ b
  PRINT -a \ b; " "; -a MOD b; " "; a / 0
  d = 7.5: e = 2
  PRINT d + e; " "; d - a; " "; d * b; " "; d / e; " "; d \ e; " "; d MOD e
  PRINT a = b; " "; a <> b; " "; a < b; " "; a <= 7; " "; d > a; " "; d >= 7.5
  PRINT (a > 1) AND (b > 1); " "; (a > 9) OR (b > 9); " "; a AND 3; " "; a OR 8; " "; a XOR 5; " "; NOT a
  a = 2147483647: a = a + 1: PRINT a
  s = 32767: s = s + 1: PRINT s
  by = 255: by = by + 2: PRINT by
  a = d: PRINT a
  a = 2.5 + 1: PRINT a
  e = -e: PRINT e; " "; -a
  e = 0: d = e / e: PRINT d; " "; d = d; " "; d <= 1; " "; d >= 1
END SUB

SUB Loops
  DIM i AS INTEGER, t AS LONG, x AS DOUBLE
  FOR i = 10 TO 1 STEP -3
    t = t + i
  NEXT
  PRINT t; " "; i
  FOR x = 0 TO 1 STEP 0.25
    t = t + 1
  NEXT
  PRINT t; " "; x
  FOR i = 1 TO 3
    DIM k AS INTEGER
    k = k + i
    PRINT k;
  NEXT
  PRINT
  FOR i = 1.5 TO 4
    PRINT i;
  NEXT
  PRINT
  i = 0
  WHILE i < 5
    i = i + 2
  WEND
  DO
    i = i - 1
  LOOP UNTIL i <= 3
  PRINT i
  IF i THEN PRINT "nonzero"
  IF x > 1.1 AND i = 3 THEN PRINT "both"
END SUB

SUB AddOne (BYREF n AS LONG)
  n = n + 1
END SUB

SUB PassByRef
  DIM c AS LONG
  c = 41
  AddOne c
  PRINT c; " "; STR$(c) + "!"; " "; LEN(STR$(c))
END SUB

FUNCTION Sq (v AS DOUBLE) AS DOUBLE
  DIM r AS DOUBLE
  r = v * v
  Sq = r
END FUNCTION

Arith
Loops
PassByRef
PRINT Sq(1.5)

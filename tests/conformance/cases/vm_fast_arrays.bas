' One-index array reads and writes as the interpreter's fused AGet / ASet
' run them (docs/vm-fast-design.md): typed arrays converting on a store
' (BYTE wraps, INTEGER truncates), an index that is a DOUBLE or text, a
' subscript outside the array (RapidQ doesn't check: what was stored there
' reads back, else the type's zero), string arrays, arrays global and
' local, an element in an expression. The answers are the interpreter's
' before (2.122) and native builds'.
DIM ga(1 TO 5) AS DOUBLE, gb(3) AS BYTE, gs(2) AS STRING, gm(2, 3) AS INTEGER
DIM i AS INTEGER, s AS DOUBLE, d AS DOUBLE

SUB Local
  DIM a(4) AS INTEGER, k AS INTEGER, t AS INTEGER
  FOR k = 0 TO 4: a(k) = k * 2.7: NEXT
  t = 0
  FOR k = 0 TO 4: t = t + a(k): NEXT
  PRINT "local "; a(0); " "; a(1); " "; a(4); " sum "; t
  a(2.7) = 99: PRINT "dbl index "; a(2)
  k = 1: a(k + 1) = a(k) + a(k + 2): PRINT "expr "; a(2)
END SUB

FOR i = 1 TO 5: ga(i) = i / 4: NEXT
s = 0
FOR i = 1 TO 5: s = s + ga(i): NEXT
PRINT "sum "; s
PRINT "elems "; ga(1); " "; ga(5); " "; ga(2) * ga(4)
FOR i = 0 TO 3: gb(i) = i * 100: NEXT
PRINT "bytes "; gb(0); " "; gb(1); " "; gb(2); " "; gb(3)
gb(1) = gb(1) + 200: PRINT "wrap "; gb(1)
gs(0) = "a": gs(1) = gs(0) + "b": gs(2) = gs(1) + gs(0)
PRINT "strings "; gs(0); " "; gs(1); " "; gs(2)
d = 2.9: PRINT "dbl idx "; ga(d)
PRINT "text idx "; ga("3")
ga(9) = 4.5: PRINT "outside "; ga(9); " "; ga(8)
gm(1, 2) = 7: gm(2, 3) = gm(1, 2) * 3: PRINT "two dims "; gm(1, 2); " "; gm(2, 3)
i = 4: ga(i) = ga(i - 1) * 2 + 1: PRINT "chain "; ga(4)
Local

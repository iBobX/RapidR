' RapidQ's expression forms (manual, Appendix C): POSTFIX (RPN) with every
' operand and operator in parentheses; NOT binding looser than a comparison
' (`a NOT > b` is NOT (a > b)); CALL inside a single-line IF before `:` and
' ELSE; a trailing comma leaving the last argument out.
SUB Pair (a AS LONG, b AS LONG)
  PRINT "pair"; a; b
END SUB
SUB Two (a AS STRING, b AS STRING)
  PRINT "<"; a; "|"; b; ">"
END SUB
A = 4 * 7 + (4 - 1)^6
B = (4) (7) (*) (4) (1) (-) (6) (^) (+)
PRINT A; " "; B
PRINT (5) (0-5) (-); " "; (2) + (3)
y = 5
IF y NOT > 0 THEN PRINT "not positive" ELSE PRINT "positive"
y = -1
IF y NOT > 0 THEN PRINT "not positive" ELSE PRINT "positive"
PRINT 3 NOT < 2; 3 NOT >= 3; 2 NOT <= 1
x = 3
IF x > 0& THEN CALL Pair(0&, -x): x = 0&
PRINT x
IF x > 0 THEN CALL Pair(5, 6) ELSE CALL Pair(7, 8)
Two "x",
DIM L AS QSTRINGLIST
L.AddItems "one",
PRINT L.ItemCount

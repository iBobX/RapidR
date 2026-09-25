' RapidQ syntax from the manual and real programs: s$[i], i++ / x += y,
' CASE x: stmt, VB scope modifiers, literal type suffixes, calling a
' FUNCTION without parentheses, "jello" style unterminated-string leniency.
FUNCTION Five AS INTEGER
  Five = 5
END FUNCTION
DIM s AS STRING
s = "hello"
PRINT s[2]; s[5]; s$[1]
x = 5 : x++ : x -= 2 : x *= 3
PRINT x
x--
x /= 2
PRINT x
t$ = "a" : t$ &= "b" : t$ += "c"
PRINT t$
SELECT CASE x
  CASE 12: PRINT "twelve"
  CASE ELSE : PRINT "other"
END SELECT
Public Const PC As Long = 7
Global Const GC = 8
Public pv As Integer
PRINT PC + GC + pv; " "; 1&; " "; 2.5!; " "; 3#
y = Five + 1
PRINT Five; " "; y; " "; Five()
PRINT "ends at the line end

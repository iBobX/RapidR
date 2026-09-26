' Declared numeric types (RapidQ manual, Appendix C): storing converts to
' the variable's type — integers round half to even and wrap to their width.
DIM n AS INTEGER, b AS BYTE, w AS WORD, sh AS SHORT, l AS LONG, dw AS DWORD
DIM d AS DOUBLE, v
n = 2.5: PRINT n
n = 3.5: PRINT n
n = -2.5: PRINT n
n = 2.7: PRINT n
n = 40000: PRINT n
b = 300: PRINT b
b = -1: PRINT b
w = 70000: PRINT w
sh = 40000: PRINT sh
l = 2147483647: l = l + 1: PRINT l
dw = -1: PRINT dw
n = "12": PRINT n + 1
d = "2.5": PRINT d * 2
v = 2.5: PRINT v

' Arrays and TYPE fields
DIM a(3) AS INTEGER
a(1) = 7.5: PRINT a(1)
TYPE TPoint
  X AS INTEGER
  Y AS DOUBLE
  Grid(2) AS BYTE
END TYPE
DIM p AS TPoint
p.X = 1.5: p.Y = 1.5: p.Grid(1) = 257
PRINT p.X; " "; p.Y; " "; p.Grid(1)

' Parameters, FUNCTION results, globals inside a SUB, local shadowing
FUNCTION Half (x AS INTEGER) AS INTEGER
  Half = x / 2
END FUNCTION
FUNCTION Twice (x AS DOUBLE) AS LONG
  RETURN x * 2
END FUNCTION
SUB Bump
  n = n + 0.5
END SUB
SUB Shadow
  DIM n AS STRING
  n = "local"
  PRINT n
END SUB
PRINT Half(7); " "; Half(5.6); " "; Twice(1.25)
n = 10: Bump: PRINT n
Shadow: PRINT n

' INPUT into a typed variable
INPUT "count? ", n
PRINT n

' A parameter shadows a global of the same name
SUB ByParam (n AS STRING)
  n = n + "!"
  PRINT n
END SUB
n = 5
ByParam "param"
PRINT n

' Array bounds, LBOUND/UBOUND per dimension, strings containing commas, numeric elements.
DIM a(1 TO 3) AS STRING
a(1) = "x,y"
a(3) = "z"
PRINT a(1) + "|" + a(2) + "|" + a(3)
PRINT STR$(LBOUND(a)) + "-" + STR$(UBOUND(a))
DIM m(2, 1 TO 4) AS INTEGER
PRINT STR$(UBOUND(m, 1)) + " " + STR$(LBOUND(m, 2)) + " " + STR$(UBOUND(m, 2))
m(2, 4) = 42
m(0, 1) = 8
PRINT STR$(m(2, 4) + m(0, 1))
DIM n(3) AS INTEGER
n(1) = 5
n(2) = n(1) * 2
PRINT STR$(n(2) + n(3))

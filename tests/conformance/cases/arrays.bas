' xfail: vm — assignment to DIM a(1 TO n) / 2-D elements fails: 'invalid assignment target'
' Arrays with explicit bounds and two dimensions.
DIM a(1 TO 3) AS INTEGER
DIM g(2, 2) AS INTEGER
DIM i AS INTEGER
FOR i = 1 TO 3
  a(i) = i * 10
NEXT i
g(1, 2) = 7
PRINT STR$(a(1) + a(2) + a(3))
PRINT STR$(g(1, 2))

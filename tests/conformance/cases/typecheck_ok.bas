' $TYPECHECK ON with everything declared: DIM, CONST, parameters, the
' FUNCTION's own name and RESULT, a suffix on a DIMmed name, and an
' undeclared variable in a $TYPECHECK OFF stretch.
$TYPECHECK ON
DIM total AS LONG, i AS INTEGER, name$ AS STRING
CONST Limit = 3
FUNCTION Twice (n AS LONG) AS LONG
  n = n * 2
  Result = n
END FUNCTION
FOR i = 1 TO Limit
  total = total + Twice(i)
NEXT
total% = total + 1
name$ = "sum"
$TYPECHECK OFF
loose = 5
$TYPECHECK ON
PRINT name$; total; loose

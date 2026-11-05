' A typed array (rapidr-codegen-rust typed_arrays) read with the wrong
' number of indices stops with the interpreter's run-time error, after
' what the program printed before it.
$APPTYPE CONSOLE
DIM g(2, 3) AS DOUBLE
g(1, 1) = 4
PRINT g(1, 1)
PRINT g(1)
PRINT "not reached"

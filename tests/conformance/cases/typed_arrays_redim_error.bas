' REDIM of a typed array (rapidr-codegen-rust typed_arrays) with an upper
' bound below the lower one stops with the interpreter's run-time error; a
' SUB's own array never DIMmed reads empty first, as a variable does.
$APPTYPE CONSOLE
DIM v(1 TO 3) AS SINGLE
DIM top AS INTEGER
SUB Empty
  IF top > 100 THEN
    DIM never(3) AS INTEGER
  END IF
  PRINT "["; never(1); "]"
END SUB
v(2) = 1.5
Empty
top = 0
REDIM v(5 TO top) AS SINGLE
PRINT "not reached"

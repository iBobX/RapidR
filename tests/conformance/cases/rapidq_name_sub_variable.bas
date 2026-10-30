' A SUB named like a global variable (any capitals): RC.EXE's `ERROR: MATRIX
' identifier already in use, try another name`. The other way round, a DIM
' after the SUB, is `ERROR: Identifier MATRIX already used, try another name`.
DIM Matrix(1 TO 3) AS INTEGER
SUB matrix
  PRINT "sub"
END SUB
PRINT "ok"

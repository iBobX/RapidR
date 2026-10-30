' A control and an array with one name (`Matrix`, `matrix`): RC.EXE keeps one
' list of a program's global names, in any capitals, and the children of a
' CREATE are in it — `ERROR: Identifier MATRIX already used, try another name`
' at the DIM (RapidR said "not an array" when the program ran).
CREATE Form AS QFORM
  CREATE Matrix AS QLABEL
    Caption = "x"
  END CREATE
END CREATE
DIM matrix(1 TO 3) AS INTEGER
PRINT "ok"

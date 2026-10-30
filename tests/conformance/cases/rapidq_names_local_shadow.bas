' A SUB's own variables and parameters may reuse the name of a global — a
' control's too (`Matrix` a QLABEL, `matrix(…)` an array in a SUB, a parameter
' `Matrix`) — and a STEP, TO, THEN or AND SUB is called by its name while a
' FOR's TO and STEP, an IF's THEN and the AND operator stay the keywords.
' The .expected is RC.EXE's output (docs/rapidq-ground-truth.md, "Names").
$APPTYPE CONSOLE
CREATE Form AS QFORM
  Caption = "x"
  CREATE Matrix AS QLABEL
    Caption = "label"
  END CREATE
END CREATE
DIM Total AS INTEGER

SUB Fill
  DIM matrix(1 TO 3) AS INTEGER
  matrix(2) = 4
  PRINT "array "; matrix(2)
END SUB

SUB ShowIt (Matrix AS INTEGER)
  PRINT "param "; Matrix
END SUB

SUB Step (A AS INTEGER, B AS INTEGER)
  PRINT "step "; A; B
END SUB

SUB To
  PRINT "to"
END SUB

SUB Then
  PRINT "then"
END SUB

SUB And
  PRINT "and"
END SUB

Fill
ShowIt 3
PRINT "control "; Matrix.Caption
Step 1, 2
CALL Step(3, 4)
To
Then
And
DIM I AS INTEGER
FOR I = 1 TO 5 STEP 2
  Total = Total + I
NEXT
PRINT "total "; Total
IF Total = 9 AND I = 7 THEN PRINT "both"
IF Total = 9 THEN
  PRINT "block"
END IF

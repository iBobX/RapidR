' $TYPECHECK ON reads that are declared: a name stored into before the
' check was on (RapidQ made it a variable), the parameters a FUNCTION was
' DECLAREd with though its own line leaves them out, a CONST, a SUB's
' parameter, a component CREATEd inside another. RC.EXE (RapidQ 2006, in
' the Windows VM: docs/rapidq-ground-truth.md) printed this output.
c = 3
$TYPECHECK ON
CONST Four = 4
DECLARE FUNCTION G (a AS INTEGER) AS INTEGER
FUNCTION G
  G = a * 2
END FUNCTION
SUB Say (t AS STRING)
  PRINT t
END SUB
CREATE F AS QFORM
  CREATE B AS QBUTTON
    Caption = "b"
  END CREATE
END CREATE
DIM L AS QSTRINGLIST
L.AddItems B.Caption
PRINT c; G(Four)
Say L.Item(0)

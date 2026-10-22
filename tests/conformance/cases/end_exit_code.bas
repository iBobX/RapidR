' END n (RapidR's): the program ends with exit code n, so a console tool can
' fail a build. An expression, in a SUB too; END IF and END SUB close their
' blocks as ever, and the lines after the END never run.
DIM n AS INTEGER
SUB Check(code AS INTEGER)
  IF code > 2 THEN
    PRINT "failing"
    END code + 4
  END IF
  PRINT "ok"
END SUB
n = 1
Check n
n = n + 2
Check n
PRINT "not reached"

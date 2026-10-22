' END alone is RapidQ's, unchanged: it ends the program with exit code 0
' (END 0 does too), and nothing after it runs.
DIM n AS INTEGER
n = 1
IF n = 1 THEN
  PRINT "plain"
END IF
END
PRINT "not reached"

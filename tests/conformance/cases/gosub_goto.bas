PRINT "start"
GOSUB Helper
PRINT "after gosub"
GOTO Done
PRINT "skipped"
Helper:
  PRINT "in helper"
RETURN
Done:
PRINT "done"

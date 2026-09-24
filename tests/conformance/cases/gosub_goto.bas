' xfail: vm, codegen — GOSUB/GOTO/labels silently dropped (ROADMAP Phase 1)
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

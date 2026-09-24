' xfail: vm, codegen — calls to undefined SUBs compile (VM: silent no-op)
PRINT "before"
NoSuchRoutine 1, 2
PRINT "after"
